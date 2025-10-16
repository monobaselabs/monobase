#!/usr/bin/env bun

/**
 * Code generation script for OpenAPI-driven API
 * Generates:
 * - TypeScript types from OpenAPI schemas
 * - Zod validators for request/response validation
 * - Routes with automatic validation
 * - Handler stubs for developers to implement
 */

import { writeFile, mkdir, exists } from 'fs/promises';
import path from 'path';
import { fileURLToPath } from 'url';
import { $ } from 'bun';
import { Glob } from 'bun';
import typespecOpenapi from '@monobase/api-spec/openapi.json';
import { auth } from './generate.auth';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const ROOT_DIR = path.join(__dirname, '..');

// Paths
const GENERATED_DIR = path.join(ROOT_DIR, 'src/generated/openapi');
const HANDLERS_DIR = path.join(ROOT_DIR, 'src/handlers');

interface OpenAPIOperation {
  operationId: string;
  summary?: string;
  description?: string;
  tags?: string[];
  security?: any[];
  parameters?: any[];
  requestBody?: any;
  responses?: any;
}

interface PathItem {
  [method: string]: OpenAPIOperation;
}

async function main() {
  console.log('🚀 Starting code generation from OpenAPI spec...\n');

  try {
    // Ensure directories exist
    await mkdir(GENERATED_DIR, { recursive: true });
    await mkdir(HANDLERS_DIR, { recursive: true });
    await mkdir(path.join(ROOT_DIR, 'src/generated/better-auth'), { recursive: true });

    // Generate Better-Auth schema
    console.log('🔐 Generating Better-Auth schema...');
    try {
      await $`npx @better-auth/cli@latest generate --config scripts/generate.auth.ts --output src/generated/better-auth/schema.ts -y`;
      console.log('   ✓ Generated Better-Auth schema\n');
    } catch (error) {
      console.error('❌ Better-Auth generation failed:', error);
      process.exit(1);
    }

    // Generate database migrations
    console.log('🗄️  Generating database migrations...');
    try {
      await $`bun run db:generate`;
      console.log('   ✓ Generated database migrations\n');
    } catch (error) {
      console.error('❌ Database migration generation failed:', error);
      process.exit(1);
    }

    // Load TypeSpec OpenAPI spec from workspace dependency
    console.log('📖 Loading TypeSpec OpenAPI spec from workspace dependency...');
    console.log(`✅ Loaded TypeSpec OpenAPI spec version ${typespecOpenapi.openapi}\n`);

    // Generate Better-Auth OpenAPI schema
    console.log('🔐 Generating Better-Auth OpenAPI schema...');
    const betterAuthOpenapi = await auth.api.generateOpenAPISchema();
    console.log('   ✓ Generated Better-Auth OpenAPI schema\n');

    // Save Better-Auth OpenAPI spec separately
    console.log('🔐 Saving Better-Auth OpenAPI spec...');
    const transformedBetterAuthSpec = transformBetterAuthSpec(betterAuthOpenapi);
    await writeFile(
      path.join(ROOT_DIR, 'src/generated/better-auth/openapi.json'),
      JSON.stringify(transformedBetterAuthSpec, null, 2)
    );
    console.log('   ✓ Saved Better-Auth OpenAPI spec\n');

    // Filter out auth/identity routes for handler generation (handled by Better-Auth)
    const nonAuthPaths = filterNonAuthPaths(typespecOpenapi.paths);
    console.log(`📋 Found ${Object.keys(nonAuthPaths).length} non-auth paths to generate handlers for\n`);

    // Generate code
    await generateTypes(typespecOpenapi);
    await generateValidators(typespecOpenapi);
    await generateRoutes(nonAuthPaths, typespecOpenapi);
    await generateRegistry(nonAuthPaths);
    await generateHandlerStubs(nonAuthPaths, typespecOpenapi);

    // Generate WebSocket handlers
    await generateWebSocketHandlers();

    console.log('\n✅ Code generation complete!');
    console.log('📝 Next steps:');
    console.log('   1. Run "bun install" to install workspace dependencies');
    console.log('   2. Implement handler functions in src/handlers/');
    console.log('   3. Run "bun dev" to start the development server');
  } catch (error) {
    console.error('❌ Generation failed:', error);
    process.exit(1);
  }
}

/**
 * Transform Better-Auth spec to add /auth prefix and update tags
 * This prepares the Better-Auth spec for standalone use
 */
function transformBetterAuthSpec(betterAuthSpec: any): any {
  // Transform Better-Auth paths to include /auth prefix and update tags
  const transformedAuthPaths: Record<string, any> = {};
  for (const [path, pathItem] of Object.entries(betterAuthSpec.paths || {})) {
    // Add /auth prefix to the path
    const newPath = `/auth${path}`;
    
    // Clone the path item and update tags in all methods
    const transformedPathItem: any = {};
    for (const [method, operation] of Object.entries(pathItem as any)) {
      const transformedOperation = { ...operation };
      
      // Set all operations to use "Auth" tag regardless of original tags
      transformedOperation.tags = ['Auth'];
      
      transformedPathItem[method] = transformedOperation;
    }
    
    transformedAuthPaths[newPath] = transformedPathItem;
  }

  // Set a single Auth tag for all Better-Auth endpoints
  const transformedAuthTags = [{
    name: 'Auth',
    description: 'Authentication, authorization, and user management endpoints powered by Better-Auth. Includes user registration, login (email/social), password management, email verification, OTP authentication, session management, account linking, and administrative operations (user creation, role management, impersonation, banning)',
  }];

  // Return transformed Better-Auth spec
  return {
    ...betterAuthSpec,
    paths: transformedAuthPaths,
    tags: transformedAuthTags,
  };
}

function filterNonAuthPaths(paths: Record<string, PathItem>): Record<string, PathItem> {
  const filtered: Record<string, PathItem> = {};
  
  for (const [path, methods] of Object.entries(paths)) {
    // Skip auth-related paths
    if (
      path.startsWith('/auth') ||
      path.startsWith('/identity') ||
      path.startsWith('/sessions') ||
      path.startsWith('/login') ||
      path.startsWith('/register') ||
      path.startsWith('/password')
    ) {
      continue;
    }
    
    filtered[path] = methods as PathItem;
  }
  
  return filtered;
}

/**
 * Check if an operation's response schema supports expand
 */
function hasExpandableResponse(operation: OpenAPIOperation, spec: any): boolean {
  // Get the success response schema (2xx)
  const successResponse = Object.entries(operation.responses || {})
    .find(([code]) => code.startsWith('2'))?.[1];

  if (!successResponse?.content?.['application/json']?.schema) {
    return false;
  }

  const schema = resolveSchema(successResponse.content['application/json'].schema, spec);

  // Check if any property has x-expandable-field metadata
  return Object.values(schema.properties || {}).some(
    (prop: any) => prop['x-expandable-field']?.opId
  );
}

/**
 * Get the response schema name for an operation
 */
function getResponseSchemaName(operation: OpenAPIOperation, spec: any): string | null {
  const successResponse = Object.entries(operation.responses || {})
    .find(([code]) => code.startsWith('2'))?.[1];

  const schema = successResponse?.content?.['application/json']?.schema;
  if (!schema) return null;

  // Handle $ref
  if (schema.$ref) {
    return schema.$ref.split('/').pop() || null;
  }

  // Handle inline schemas (less common)
  return null;
}

/**
 * Resolve a schema reference or return the schema itself
 */
function resolveSchema(schema: any, spec: any): any {
  if (schema.$ref) {
    const refPath = schema.$ref.replace('#/components/schemas/', '');
    return spec.components?.schemas?.[refPath] || {};
  }
  return schema;
}

async function generateTypes(spec: any) {
  console.log('🔧 Using TypeScript types from workspace dependency...');
  
  // Use types from @monobase/api-spec workspace dependency
  const content = `/**
 * TypeScript types from @monobase/api-spec workspace dependency
 * Generated from OpenAPI spec in specs/api
 */

// Import types from workspace dependency
export * from '@monobase/api-spec/types';
`;

  await writeFile(path.join(GENERATED_DIR, 'types.ts'), content);
  console.log('   ✓ Generated types.ts using workspace dependency');
}

function topologicalSortSchemas(schemas: Record<string, any>): [string, any][] {
  const visited = new Set<string>();
  const visiting = new Set<string>();
  const result: [string, any][] = [];
  
  function extractReferences(obj: any): string[] {
    const refs: string[] = [];
    
    if (typeof obj !== 'object' || obj === null) {
      return refs;
    }
    
    if (obj.$ref && typeof obj.$ref === 'string') {
      const match = obj.$ref.match(/#\/components\/schemas\/(.+)$/);
      if (match) {
        refs.push(match[1]);
      }
    }
    
    if (Array.isArray(obj)) {
      for (const item of obj) {
        refs.push(...extractReferences(item));
      }
    } else {
      for (const value of Object.values(obj)) {
        refs.push(...extractReferences(value));
      }
    }
    
    return refs;
  }
  
  function visit(name: string): void {
    if (visited.has(name)) {
      return;
    }
    
    if (visiting.has(name)) {
      // Circular dependency detected, but we'll continue
      // The generated code should handle this gracefully
      console.warn(`⚠️  Circular dependency detected involving schema: ${name}`);
      return;
    }
    
    const schema = schemas[name];
    if (!schema) {
      return; // Schema doesn't exist, skip
    }
    
    visiting.add(name);
    
    // Visit all dependencies first
    const refs = extractReferences(schema);
    for (const ref of refs) {
      if (schemas[ref]) { // Only visit if schema exists
        visit(ref);
      }
    }
    
    visiting.delete(name);
    visited.add(name);
    result.push([name, schema]);
  }
  
  // Visit all schemas
  for (const name of Object.keys(schemas)) {
    visit(name);
  }
  
  return result;
}

async function generateValidators(spec: any) {
  console.log('🔧 Generating Zod validators...');
  
  const validators: string[] = [
    "import { z } from 'zod';",
    "import ISO6391 from 'iso-639-1';",
    "// Import browser version to bypass entry-node.js dynamic requires",
    "import countries from 'i18n-iso-countries/index';",
    "import enLocale from 'i18n-iso-countries/langs/en.json';",
    "import { getTimeZones } from '@vvo/tzdb';",
    "import { isValidPhoneNumber } from 'libphonenumber-js';",
    '',
    '// Register English locale for country code validation',
    'countries.registerLocale(enLocale);',
    '',
    '// Generated Zod validators from OpenAPI spec',
    '',
    '// Healthcare validation helpers',
    'const validateNPI = (npi: string): boolean => {',
    '  // NPI validation algorithm (Luhn algorithm)',
    '  const digits = npi.split("").map(Number);',
    '  let sum = 0;',
    '  for (let i = 0; i < 9; i++) {',
    '    let digit = digits[i];',
    '    if (digit === undefined) return false; // Type guard for undefined',
    '    if (i % 2 === 0) {',
    '      digit *= 2;',
    '      if (digit > 9) digit -= 9;',
    '    }',
    '    sum += digit;',
    '  }',
    '  const lastDigit = digits[9];',
    '  if (lastDigit === undefined) return false; // Type guard for undefined',
    '  return (10 - (sum % 10)) % 10 === lastDigit;',
    '};',
    '',
    'const containsPHI = (value: string): boolean => {',
    '  // Basic PHI detection patterns',
    '  const phiPatterns = [',
    '    /\\b\\d{3}-\\d{2}-\\d{4}\\b/, // SSN',
    '    /\\b\\d{4}[\\s-]?\\d{4}[\\s-]?\\d{4}[\\s-]?\\d{4}\\b/, // Credit card',
    '    /\\b[A-Z]{2}\\d{6}[A-Z]\\b/, // Medical record patterns',
    '  ];',
    '  return phiPatterns.some(pattern => pattern.test(value));',
    '};',
    '',
    '// International data validation helpers',
    'const validateLanguageCode = (code: string): boolean => {',
    '  return ISO6391.validate(code);',
    '};',
    '',
    'const validateCountryCode = (code: string): boolean => {',
    '  return countries.isValid(code);',
    '};',
    '',
    'const validatePhoneNumber = (phone: string): boolean => {',
    '  try {',
    '    // libphonenumber-js validates E.164 format and country-specific rules',
    '    return isValidPhoneNumber(phone);',
    '  } catch {',
    '    return false;',
    '  }',
    '};',
    '',
    'const timezoneNames = getTimeZones().map(tz => tz.name);',
    'const validateTimezone = (tz: string): boolean => {',
    '  return timezoneNames.includes(tz);',
    '};',
    '',
  ];

  // Track generated validator names to prevent duplicates
  const generatedNames = new Set<string>();

  // Generate validators for each schema in dependency order
  if (spec.components?.schemas) {
    const sortedSchemas = topologicalSortSchemas(spec.components.schemas);
    for (const [name, schema] of sortedSchemas) {
      // Sanitize the name to match what generateZodSchema will produce
      const cleanName = name
        .split('.')
        .map((part, index) => index === 0 ? part : capitalize(part))
        .join('');
      const schemaName = `${cleanName}Schema`;
      
      if (!generatedNames.has(schemaName)) {
        generatedNames.add(schemaName);
        validators.push(generateZodSchema(name, schema as any));
        validators.push('');
      }
    }
  }

  // Generate parameter validators for operations
  for (const [path, methods] of Object.entries(spec.paths)) {
    for (const [method, operation] of Object.entries(methods as PathItem)) {
      if (operation.operationId) {
        // Path parameters - resolve $ref parameters first
        const pathParams = operation.parameters?.map((p: any) => resolveParameter(p, spec)).filter((p: any) => p.in === 'path');
        if (pathParams?.length) {
          const paramsName = `${capitalize(operation.operationId)}Params`;
          if (!generatedNames.has(paramsName)) {
            generatedNames.add(paramsName);
            validators.push(`export const ${paramsName} = z.object({`);
            for (const param of pathParams) {
              const zodType = convertParameterToZod(param);
              validators.push(`  ${param.name}: ${zodType},`);
            }
            validators.push('});');
            validators.push(`export type ${paramsName} = z.infer<typeof ${paramsName}>;`);
            validators.push('');
          }
        }

        // Query parameters - resolve $ref parameters first
        const queryParams = operation.parameters?.map((p: any) => resolveParameter(p, spec)).filter((p: any) => p.in === 'query');
        if (queryParams?.length) {
          const queryName = `${capitalize(operation.operationId)}Query`;
          if (!generatedNames.has(queryName)) {
            generatedNames.add(queryName);
            validators.push(`export const ${queryName} = z.object({`);
            for (const param of queryParams) {
              const zodType = convertParameterToZod(param);
              validators.push(`  ${param.name}: ${zodType},`);
            }
            validators.push('});');
            validators.push(`export type ${queryName} = z.infer<typeof ${queryName}>;`);
            validators.push('');
          }
        }

        // Request body
        if (operation.requestBody) {
          const bodyName = `${capitalize(operation.operationId)}Body`;
          if (!generatedNames.has(bodyName)) {
            generatedNames.add(bodyName);
            const bodyValidator = generateRequestBodyValidator(operation.requestBody);
            validators.push(`export const ${bodyName} = ${bodyValidator};`);
            validators.push(`export type ${bodyName} = z.infer<typeof ${bodyName}>;`);
            validators.push('');
          }
        }

        // Response validators
        if (operation.responses) {
          for (const [statusCode, response] of Object.entries(operation.responses)) {
            if (statusCode === 'default' || statusCode.startsWith('2')) {
              const responseName = `${capitalize(operation.operationId)}Response`;
              if (!generatedNames.has(responseName)) {
                generatedNames.add(responseName);
                const responseValidator = generateResponseValidator(response);
                validators.push(`export const ${responseName} = ${responseValidator};`);
                validators.push('');
              }
              break; // Only generate for first 2xx response
            }
          }
        }
      }
    }
  }

  await writeFile(path.join(GENERATED_DIR, 'validators.ts'), validators.join('\n'));
  console.log('   ✓ Generated validators.ts');
}

async function generateRoutes(paths: Record<string, PathItem>, spec: any) {
  console.log('🔧 Generating routes...');
  
  const routes: string[] = [
    "import type { Hono, Handler } from 'hono';",
    "import { zValidator } from '@hono/zod-validator';",
    "import type { Variables } from '@/types/app';",
    "import * as validators from './validators';",
    "import { registry } from './registry';",
    "import { authMiddleware } from '@/middleware/auth';",
    "import { validationErrorHandler } from '@/middleware/validation';",
    "import { createExpandMiddleware } from '@/middleware/expand';",
    '',
    'export function registerRoutes(app: Hono<{ Variables: Variables }>) {',
  ];

  for (const [path, methods] of Object.entries(paths)) {
    for (const [method, operation] of Object.entries(methods)) {
      if (!operation.operationId) continue;

      const honoPath = path.replace(/{([^}]+)}/g, ':$1'); // Convert {id} to :id
      const hasAuth = !!operation.security?.length;
      
      // Check if authentication is optional (contains both bearerAuth and empty object)
      const isOptionalAuth = hasAuth && operation.security?.some((sec: any) => 
        Object.keys(sec).length === 0
      ) && operation.security?.some((sec: any) => 
        sec.bearerAuth !== undefined
      );
      
      // Extract x-security-required-roles from the operation
      const requiredRoles = (operation as any)['x-security-required-roles'] as string[] | undefined;
      
      routes.push(`  // ${operation.summary || operation.operationId}`);
      routes.push(`  app.${method}('${honoPath}',`);
      
      // Add auth middleware with roles if needed
      if (hasAuth) {
        if (isOptionalAuth) {
          // Optional authentication - no roles enforcement since it's optional
          routes.push('    authMiddleware({ required: false }),');
        } else if (requiredRoles && requiredRoles.length > 0) {
          // Required authentication with specific roles
          const rolesArray = requiredRoles.map(role => `"${role}"`).join(', ');
          routes.push(`    authMiddleware({ roles: [${rolesArray}] }),`);
        } else {
          // Required authentication without specific roles
          routes.push('    authMiddleware(),');
        }
      }
      
      // Add validators
      const pathParams = operation.parameters?.map((p: any) => resolveParameter(p, spec)).filter((p: any) => p.in === 'path');
      if (pathParams?.length) {
        routes.push(`    zValidator('param', validators.${capitalize(operation.operationId)}Params, validationErrorHandler),`);
      }

      const queryParams = operation.parameters?.map((p: any) => resolveParameter(p, spec)).filter((p: any) => p.in === 'query');
      if (queryParams?.length) {
        routes.push(`    zValidator('query', validators.${capitalize(operation.operationId)}Query, validationErrorHandler),`);
      }

      if (operation.requestBody) {
        routes.push(`    zValidator('json', validators.${capitalize(operation.operationId)}Body, validationErrorHandler),`);
      }

      // Check if operation supports expand
      const supportsExpand = hasExpandableResponse(operation, spec);
      const responseSchema = getResponseSchemaName(operation, spec);

      // Add expand middleware BEFORE handler if response schema supports it
      if (supportsExpand && responseSchema) {
        routes.push(`    createExpandMiddleware("${responseSchema}"),`);
      }

      // Handler
      routes.push(`    registry.${operation.operationId} as unknown as Handler`);
      routes.push('  );');
      routes.push('');
    }
  }

  routes.push('}');

  await writeFile(path.join(GENERATED_DIR, 'routes.ts'), routes.join('\n'));
  console.log('   ✓ Generated routes.ts');
}

async function generateRegistry(paths: Record<string, PathItem>) {
  console.log('🔧 Generating handler registry...');
  
  const imports: string[] = [];
  const registryEntries: string[] = [];
  const operationsByModule = new Map<string, string[]>();

  // Group operations by module (tag)
  for (const [path, methods] of Object.entries(paths)) {
    for (const [method, operation] of Object.entries(methods)) {
      if (!operation.operationId) continue;
      
      const module = operation.tags?.[0]?.toLowerCase() || 'default';
      if (!operationsByModule.has(module)) {
        operationsByModule.set(module, []);
      }
      operationsByModule.get(module)!.push(operation.operationId);
    }
  }

  // Generate static imports and registry entries
  for (const [module, operations] of operationsByModule) {
    registryEntries.push(`  // ${capitalize(module)} handlers`);
    for (const operationId of operations) {
      // Add static import
      imports.push(`import { ${operationId} } from '../../handlers/${module}/${operationId}';`);
      // Add registry entry as direct function reference
      registryEntries.push(`  ${operationId},`);
    }
    registryEntries.push('');
  }

  const registry: string[] = [
    '/**',
    ' * Handler registry - maps operationIds to handler functions',
    ' * This file is regenerated on each run',
    ' */',
    '',
    ...imports,
    '',
    'export const registry = {',
    ...registryEntries,
    '};',
  ];

  await writeFile(path.join(GENERATED_DIR, 'registry.ts'), registry.join('\n'));
  console.log('   ✓ Generated registry.ts');
}

async function generateHandlerStubs(paths: Record<string, PathItem>, spec: any) {
  console.log('🔧 Generating handler stubs...');
  
  let generated = 0;
  let skipped = 0;

  for (const [apiPath, methods] of Object.entries(paths)) {
    for (const [method, operation] of Object.entries(methods)) {
      if (!operation.operationId) continue;
      
      const module = operation.tags?.[0]?.toLowerCase() || 'default';
      const handlerDir = path.join(HANDLERS_DIR, module);
      const handlerPath = path.join(handlerDir, `${operation.operationId}.ts`);
      
      // Check if handler already exists
      if (await exists(handlerPath)) {
        skipped++;
        continue;
      }
      
      // Create directory if needed
      await mkdir(handlerDir, { recursive: true });
      
      // Generate handler stub
      const stub = generateHandlerStub(operation, apiPath, method, spec);
      await writeFile(handlerPath, stub);
      generated++;
    }
  }

  console.log(`   ✓ Generated ${generated} new handler stubs`);
  if (skipped > 0) {
    console.log(`   ⏭️  Skipped ${skipped} existing handlers`);
  }
}

function generateHandlerStub(operation: OpenAPIOperation, path: string, method: string, spec: any): string {
  const hasAuth = !!operation.security?.length;
  const isOptionalAuth = hasAuth && operation.security?.some((sec: any) =>
    Object.keys(sec).length === 0
  ) && operation.security?.some((sec: any) =>
    sec.bearerAuth !== undefined
  );
  const hasParams = operation.parameters?.map((p: any) => resolveParameter(p, spec)).some((p: any) => p.in === 'path');
  const hasQuery = operation.parameters?.map((p: any) => resolveParameter(p, spec)).some((p: any) => p.in === 'query');
  const hasBody = !!operation.requestBody;
  
  // Extract x-security-required-roles to check for ownership validation needs
  const requiredRoles = (operation as any)['x-security-required-roles'] as string[] | undefined;
  const hasOwnershipRoles = requiredRoles?.some(role => role.includes(':owner'));
  
  // Generate ownership validation comments
  let ownershipComments = '';
  if (hasOwnershipRoles) {
    const ownershipRolesList = requiredRoles!
      .filter(role => role.includes(':owner'))
      .map(role => `'${role}'`)
      .join(', ');
    
    ownershipComments = `
  // Note: This endpoint requires ownership validation for ${ownershipRolesList}
  // Check that the authenticated user owns the requested resource
  // Example:
  // if (session.user.role === 'patient' && params.patientId !== session.user.id) {
  //   throw new ForbiddenError('You can only access your own resources');
  // }`;
  }

  // Generate ValidatedContext type parameters
  const typeParams: string[] = [];
  
  // TJson - request body type
  if (hasBody) {
    const bodyType = `${capitalize(operation.operationId)}Body`;
    typeParams.push(bodyType);
  } else {
    typeParams.push('never');
  }
  
  // TQuery - query parameters type
  if (hasQuery) {
    const queryType = `${capitalize(operation.operationId)}Query`;
    typeParams.push(queryType);
  } else {
    typeParams.push('never');
  }
  
  // TParam - path parameters type
  if (hasParams) {
    const paramType = `${capitalize(operation.operationId)}Params`;
    typeParams.push(paramType);
  } else {
    typeParams.push('never');
  }
  
  const contextType = typeParams.every(t => t === 'never') 
    ? 'BaseContext' 
    : `ValidatedContext<${typeParams.join(', ')}>`;
  
  // Generate type imports - only import the context type from @/types/app
  const contextImport = contextType.startsWith('ValidatedContext') 
    ? 'ValidatedContext' 
    : 'BaseContext';

  return `import type { ${contextImport} } from '@/types/app';
import { db } from '@/core/database';
import { 
  UnauthorizedError,
  ForbiddenError,
  NotFoundError,
  ValidationError,
  BusinessLogicError
} from '@/core/errors';
${hasBody || hasQuery || hasParams ? `import type { ${[
    hasBody && `${capitalize(operation.operationId)}Body`,
    hasQuery && `${capitalize(operation.operationId)}Query`, 
    hasParams && `${capitalize(operation.operationId)}Params`
  ].filter(Boolean).join(', ')} } from '@/generated/openapi/validators';` : ''}

/**
 * ${operation.summary || operation.operationId}
 * 
 * Path: ${method.toUpperCase()} ${path}
 * OperationId: ${operation.operationId}
 */
export async function ${operation.operationId}(
  ctx: ${contextType}
): Promise<Response> {
  ${isOptionalAuth ? `// Optional authentication - check if user is authenticated
  const session = ctx.get('session'); // May be null
  const user = ctx.get('user'); // May be null
  
  if (user) {
    // User is authenticated - can return additional data
    // Example: include sensitive fields, personalized content, etc.
  } else {
    // User is not authenticated - return public data only
  }` : hasAuth ? `// Get authenticated session from Better-Auth
  const session = ctx.get('session');
  if (!session) {
    throw new UnauthorizedError();
  }${ownershipComments}` : '// Public endpoint - no auth required'}
  
  ${hasParams ? `// Extract validated parameters
  const params = ctx.req.valid('param');` : ''}
  ${hasQuery ? `// Extract validated query parameters
  const query = ctx.req.valid('query');` : ''}
  ${hasBody ? `// Extract validated request body
  const body = ctx.req.valid('json');` : ''}
  
  // TODO: Implement business logic
  // Examples of throwing errors:
  // throw new UnauthorizedError();
  // throw new ForbiddenError('You do not have access to this resource');
  // throw new NotFoundError('Resource');
  // throw new ValidationError('Invalid input');
  // throw new BusinessLogicError('Business rule violated', 'BUSINESS_ERROR');
  
  throw new Error('Not implemented: ${operation.operationId}');
}`;
}

function generateZodSchema(name: string, schema: any): string {
  // Sanitize the name to be a valid JavaScript identifier
  // Convert dots to camelCase (e.g., "Http.NoContentResponse" -> "HttpNoContentResponse")
  const cleanName = name
    .split('.')
    .map((part, index) => index === 0 ? part : capitalize(part))
    .join('');
  
  const zodSchema = convertOpenAPIToZod(schema);
  return `export const ${cleanName}Schema = ${zodSchema};`;
}

function convertOpenAPIToZod(schema: any): string {
  if (!schema) return 'z.unknown()';

  // Handle $ref references
  if (schema.$ref) {
    const refName = schema.$ref.split('/').pop();
    const cleanRefName = refName
      .split('.')
      .map((part, index) => index === 0 ? part : capitalize(part))
      .join('');
    let result = `${cleanRefName}Schema`;
    // Handle nullable refs
    if (schema.nullable) {
      result = `z.union([${result}, z.null()])`;
    }
    return result;
  }

  // Handle nullable types early
  if (schema.nullable === true) {
    // Create a copy without nullable to process the base type
    const baseSchema = { ...schema };
    delete baseSchema.nullable;
    const baseZodSchema = convertOpenAPIToZod(baseSchema);
    return `z.union([${baseZodSchema}, z.null()])`;
  }

  // Handle schemas with both allOf and properties/type
  // This is common for models that extend BaseEntity
  if (schema.allOf && (schema.properties || schema.type)) {
    // Special case: If allOf contains only a single $ref and type is "object" without properties,
    // TypeSpec is just wrapping a ref unnecessarily - return the ref directly
    if (schema.allOf.length === 1 && 
        schema.allOf[0].$ref && 
        schema.type === 'object' && 
        !schema.properties) {
      // Just use the reference directly, no need for intersection with empty object
      return convertOpenAPIToZod(schema.allOf[0]);
    }
    
    // Process allOf to get base schemas
    const allOfSchemas = schema.allOf.map(s => convertOpenAPIToZod(s));
    const baseSchema = allOfSchemas.length === 1 
      ? allOfSchemas[0] 
      : `z.intersection(${allOfSchemas.join(', ')})`;
    
    // Process direct properties/type by creating a copy without allOf
    const schemaWithoutAllOf = { ...schema };
    delete schemaWithoutAllOf.allOf;
    const directSchema = convertOpenAPIToZod(schemaWithoutAllOf);
    
    // If directSchema is just an empty object or record, use baseSchema only
    if (directSchema === 'z.object({})' || directSchema === 'z.record(z.string(), z.unknown())') {
      return baseSchema;
    }
    
    // Combine base schema from allOf with direct schema
    return `z.intersection(${baseSchema}, ${directSchema})`;
  }

  // Handle allOf (composition) - for schemas with only allOf
  if (schema.allOf) {
    if (schema.allOf.length === 1) {
      return convertOpenAPIToZod(schema.allOf[0]);
    }
    const schemas = schema.allOf.map(s => convertOpenAPIToZod(s));
    return `z.intersection(${schemas.join(', ')})`;
  }

  // Handle oneOf (union)
  if (schema.oneOf) {
    const schemas = schema.oneOf.map(s => convertOpenAPIToZod(s));
    return `z.union([${schemas.join(', ')}])`;
  }

  // Handle anyOf (union)
  if (schema.anyOf) {
    const schemas = schema.anyOf.map(s => convertOpenAPIToZod(s));
    return `z.union([${schemas.join(', ')}])`;
  }

  switch (schema.type) {
    case 'string':
      return convertStringSchema(schema);
    case 'number':
    case 'integer':
      return convertNumberSchema(schema);
    case 'boolean':
      return 'z.boolean()';
    case 'array':
      return convertArraySchema(schema);
    case 'object':
      return convertObjectSchema(schema);
    default:
      return 'z.unknown()';
  }
}

function convertStringSchema(schema: any): string {
  let base = 'z.string()';
  
  // Handle format constraints
  if (schema.format) {
    switch (schema.format) {
      case 'uuid':
        base = 'z.string().uuid()';
        break;
      case 'email':
        base = 'z.string().email()';
        break;
      case 'date':
        // Validate both format and calendar validity (catches impossible dates like 2000-02-30)
        base = 'z.string().regex(/^\\d{4}-\\d{2}-\\d{2}$/).refine(val => { const parsed = new Date(val + "T00:00:00Z"); return !isNaN(parsed.getTime()) && parsed.toISOString().split("T")[0] === val; }, { message: "Invalid calendar date" })';
        break;
      case 'date-time':
        base = 'z.string().datetime().transform((str) => new Date(str))';
        break;
      case 'uri':
      case 'url':
        base = 'z.string().url()';
        break;
      case 'password':
        base = 'z.string().min(8)';
        break;
      // Healthcare-specific formats
      case 'npi':
        base = 'z.string().regex(/^\\d{10}$/).refine(val => validateNPI(val), { message: "Invalid NPI number" })';
        break;
      case 'mrn':
        base = 'z.string().regex(/^[A-Z0-9]{6,12}$/).refine(val => val.length >= 6, { message: "MRN must be at least 6 characters" })';
        break;
      case 'phone':
        base = 'z.string().regex(/^\\+[1-9]\\d{1,14}$/).refine(val => validatePhoneNumber(val), { message: "Invalid phone number in E.164 format" })';
        break;
      case 'ssn':
        // SSN should be properly masked and validated
        base = 'z.string().regex(/^\\d{3}-\\d{2}-\\d{4}$/).or(z.string().regex(/^XXX-XX-\\d{4}$/))';
        break;
      // International data formats
      case 'iso-639-1':
        base = 'z.string().regex(/^[a-z]{2}$/).refine(val => validateLanguageCode(val), { message: "Invalid ISO 639-1 language code" })';
        break;
      case 'iso-3166-1-alpha-2':
        base = 'z.string().regex(/^[A-Z]{2}$/).refine(val => validateCountryCode(val), { message: "Invalid ISO 3166-1 country code" })';
        break;
      case 'iana-timezone':
        base = 'z.string().regex(/^[A-Za-z_]+\\/[A-Za-z_]+$/).refine(val => validateTimezone(val), { message: "Invalid IANA timezone identifier" })';
        break;
      default:
        base = 'z.string()';
    }
  }
  
  // Handle pattern (regex)
  if (schema.pattern && !schema.format) {
    base += `.regex(/${schema.pattern}/)`;
  }
  
  // Handle length constraints
  if (schema.minLength !== undefined) {
    base += `.min(${schema.minLength})`;
  }
  if (schema.maxLength !== undefined) {
    base += `.max(${schema.maxLength})`;
  }
  
  // Handle enum values
  if (schema.enum) {
    const enumValues = schema.enum.map(v => `"${v}"`).join(', ');
    base = `z.enum([${enumValues}])`;
  }
  
  // Add HIPAA compliance for certain fields
  if (schema['x-hipaa-protected']) {
    base = `${base}.refine(val => !containsPHI(val), { message: "Field contains PHI and must be properly encrypted" })`;
  }
  
  return base;
}

function convertNumberSchema(schema: any): string {
  let base = schema.type === 'integer' ? 'z.number().int()' : 'z.number()';
  
  // Handle numeric constraints
  if (schema.minimum !== undefined) {
    if (schema.exclusiveMinimum) {
      base += `.gt(${schema.minimum})`;
    } else {
      base += `.gte(${schema.minimum})`;
    }
  }
  if (schema.maximum !== undefined) {
    if (schema.exclusiveMaximum) {
      base += `.lt(${schema.maximum})`;
    } else {
      base += `.lte(${schema.maximum})`;
    }
  }
  
  return base;
}

function convertArraySchema(schema: any): string {
  let base = 'z.array(';
  
  if (schema.items) {
    base += convertOpenAPIToZod(schema.items);
  } else {
    base += 'z.unknown()';
  }
  
  base += ')';
  
  // Handle array length constraints
  if (schema.minItems !== undefined) {
    base += `.min(${schema.minItems})`;
  }
  if (schema.maxItems !== undefined) {
    base += `.max(${schema.maxItems})`;
  }
  
  return base;
}

function convertObjectSchema(schema: any): string {
  if (!schema.properties) {
    // If no properties defined, it's a generic object
    // Zod v4 requires z.record to have both key and value types
    return schema.additionalProperties === false ? 'z.object({})' : 'z.record(z.string(), z.unknown())';
  }
  
  const properties: string[] = [];
  const required = schema.required || [];
  
  for (const [propName, propSchema] of Object.entries(schema.properties)) {
    let propZod = convertOpenAPIToZod(propSchema);
    
    // Make optional if not required
    if (!required.includes(propName)) {
      propZod += '.optional()';
    }
    
    properties.push(`  ${propName}: ${propZod}`);
  }
  
  let result = `z.object({\n${properties.join(',\n')}\n})`;
  
  // Handle additional properties
  if (schema.additionalProperties === false) {
    result += '.strict()';
  } else if (schema.additionalProperties === true || schema.additionalProperties) {
    // Use .passthrough() instead of z.intersection() to avoid Zod v4 bug
    // This allows additional properties while maintaining type safety for defined properties
    result += '.passthrough()';
  }
  
  return result;
}

function convertParameterToZod(param: any): string {
  let zodType: string;
  
  // Use schema if available, otherwise infer from type
  if (param.schema) {
    zodType = convertOpenAPIToZod(param.schema);
  } else {
    // Fallback to basic types for older OpenAPI specs
    switch (param.type || 'string') {
      case 'string':
        zodType = param.format === 'uuid' ? 'z.string().uuid()' : 'z.string()';
        break;
      case 'integer':
        zodType = 'z.coerce.number().int()';
        break;
      case 'number':
        zodType = 'z.coerce.number()';
        break;
      case 'boolean':
        zodType = 'z.coerce.boolean()';
        break;
      case 'array':
        // Handle comma-separated arrays in query params
        zodType = 'z.string().transform(val => val.split(",").filter(Boolean))';
        break;
      default:
        zodType = 'z.string()';
    }
  }
  
  // Add query parameter specific transformations
  if (param.in === 'query') {
    // Detect union types with enum arrays (e.g., EmailQueueStatus | EmailQueueStatus[])
    // Pattern: z.union([EnumSchema, z.array(EnumSchema)])
    // Check for both oneOf and anyOf (TypeSpec can generate either)
    const unionOptions = param.schema?.oneOf || param.schema?.anyOf;
    const isEnumArrayUnion = unionOptions && 
      unionOptions.length === 2 &&
      unionOptions.some((opt: any) => opt.$ref) &&
      unionOptions.some((opt: any) => opt.type === 'array' && opt.items?.$ref);
    
    if (isEnumArrayUnion) {
      // Extract the enum schema name from the single value option
      const enumRef = unionOptions.find((opt: any) => opt.$ref).$ref;
      const enumSchemaName = enumRef.split('/').pop();
      const cleanEnumName = enumSchemaName
        .split('.')
        .map((part: string, index: number) => index === 0 ? part : capitalize(part))
        .join('');
      
      // Generate union with CSV transformation as third option
      // This allows: single enum value, actual array, OR comma-separated string
      zodType = `z.union([${cleanEnumName}Schema, z.array(${cleanEnumName}Schema), z.string().transform(val => val.split(",").map(s => s.trim())).pipe(z.array(${cleanEnumName}Schema))])`;
    }
    // Handle simple array query parameters (e.g., ?tags=a,b,c)
    else if (param.schema?.type === 'array' || param.type === 'array') {
      zodType = 'z.string().transform(val => val.split(",").filter(Boolean))';
    }
    // Handle number coercion for query params (with constraints preserved)
    else if (param.schema?.type === 'number' || param.type === 'number') {
      // Start with coerced number, then apply constraints from schema
      let baseType = 'z.coerce.number()';
      if (param.schema) {
        // Apply numeric constraints
        if (param.schema.minimum !== undefined) {
          if (param.schema.exclusiveMinimum) {
            baseType += `.gt(${param.schema.minimum})`;
          } else {
            baseType += `.gte(${param.schema.minimum})`;
          }
        }
        if (param.schema.maximum !== undefined) {
          if (param.schema.exclusiveMaximum) {
            baseType += `.lt(${param.schema.maximum})`;
          } else {
            baseType += `.lte(${param.schema.maximum})`;
          }
        }
      }
      zodType = baseType;
    }
    else if (param.schema?.type === 'integer' || param.type === 'integer') {
      // Start with coerced integer, then apply constraints from schema
      let baseType = 'z.coerce.number().int()';
      if (param.schema) {
        // Apply numeric constraints
        if (param.schema.minimum !== undefined) {
          if (param.schema.exclusiveMinimum) {
            baseType += `.gt(${param.schema.minimum})`;
          } else {
            baseType += `.gte(${param.schema.minimum})`;
          }
        }
        if (param.schema.maximum !== undefined) {
          if (param.schema.exclusiveMaximum) {
            baseType += `.lt(${param.schema.maximum})`;
          } else {
            baseType += `.lte(${param.schema.maximum})`;
          }
        }
      }
      zodType = baseType;
    }
    // Handle boolean coercion for query params
    else if (param.schema?.type === 'boolean' || param.type === 'boolean') {
      zodType = 'z.coerce.boolean()';
    }
  }
  
  // Make optional if not required
  if (!param.required) {
    zodType += '.optional()';
  }
  
  return zodType;
}

function generateRequestBodyValidator(requestBody: any): string {
  if (!requestBody.content) {
    return 'z.unknown()';
  }
  
  // Handle different content types
  const contentTypes = Object.keys(requestBody.content);
  
  // Prefer application/json
  if (requestBody.content['application/json']) {
    const schema = requestBody.content['application/json'].schema;
    return convertOpenAPIToZod(schema);
  }
  
  // Handle multipart/form-data
  if (requestBody.content['multipart/form-data']) {
    const schema = requestBody.content['multipart/form-data'].schema;
    return convertOpenAPIToZod(schema);
  }
  
  // Handle application/x-www-form-urlencoded
  if (requestBody.content['application/x-www-form-urlencoded']) {
    const schema = requestBody.content['application/x-www-form-urlencoded'].schema;
    return convertOpenAPIToZod(schema);
  }
  
  // Handle text/plain
  if (requestBody.content['text/plain']) {
    return 'z.string()';
  }
  
  // Default to first available content type
  if (contentTypes.length > 0) {
    const firstContentType = contentTypes[0];
    const schema = requestBody.content[firstContentType].schema;
    if (schema) {
      return convertOpenAPIToZod(schema);
    }
  }
  
  // Fallback
  return 'z.unknown()';
}

function generateResponseValidator(response: any): string {
  if (!response.content) {
    // No content responses (204, etc.)
    return 'z.void()';
  }
  
  // Handle different content types
  const contentTypes = Object.keys(response.content);
  
  // Prefer application/json
  if (response.content['application/json']) {
    const schema = response.content['application/json'].schema;
    return convertOpenAPIToZod(schema);
  }
  
  // Handle text/plain
  if (response.content['text/plain']) {
    return 'z.string()';
  }
  
  // Handle application/octet-stream or binary
  if (response.content['application/octet-stream'] || response.content['application/pdf']) {
    return 'z.instanceof(ArrayBuffer).or(z.instanceof(Blob))';
  }
  
  // Default to first available content type
  if (contentTypes.length > 0) {
    const firstContentType = contentTypes[0];
    const schema = response.content[firstContentType].schema;
    if (schema) {
      return convertOpenAPIToZod(schema);
    }
  }
  
  // Fallback
  return 'z.unknown()';
}

function resolveParameter(param: any, spec: any): any {
  // If it's a $ref parameter, resolve it from components.parameters
  if (param.$ref) {
    const refPath = param.$ref.replace('#/components/parameters/', '');
    const resolvedParam = spec.components?.parameters?.[refPath];
    if (resolvedParam) {
      return resolvedParam;
    }
  }
  
  // Return the parameter as-is if it's not a reference
  return param;
}

function capitalize(str: string): string {
  return str.charAt(0).toUpperCase() + str.slice(1);
}

async function generateWebSocketHandlers() {
  console.log('🔌 Generating WebSocket handlers...');

  // Create WebSocket generated directory
  const WS_GENERATED_DIR = path.join(ROOT_DIR, 'src/generated/websocket');
  await mkdir(WS_GENERATED_DIR, { recursive: true });

  // Find all ws.*.ts files in handlers directory
  const glob = new Glob('**/ws.*.ts');
  const wsFiles: string[] = [];

  for await (const file of glob.scan({ cwd: path.join(ROOT_DIR, 'src/handlers'), absolute: false })) {
    wsFiles.push(file);
  }

  if (wsFiles.length === 0) {
    console.log('   ⚠️  No WebSocket handlers found (pattern: **/ws.*.ts)');
    console.log('   ✓ Generated empty WebSocket registry\n');

    // Generate empty registry
    const emptyRegistry = `/**
 * WebSocket handler registry - Auto-generated
 * DO NOT EDIT - Generated by scripts/generate.ts
 */

import type { App } from '@/types/app';

export const wsRegistry = {};

export function registerRoutes(app: App) {
  // No WebSocket handlers registered yet
  const ws = app.ws;
  app.logger.debug('No WebSocket handlers to register');
}
`;
    await writeFile(path.join(WS_GENERATED_DIR, 'registry.ts'), emptyRegistry);
    return;
  }

  console.log(`   📋 Found ${wsFiles.length} WebSocket handler(s)`);

  const imports: string[] = [];
  const registryEntries: string[] = [];

  for (const file of wsFiles) {
    // Extract config name from file
    const fileName = path.basename(file, '.ts');
    const modulePath = path.dirname(file);

    // Convert ws.feature-name.ts to featureName_config
    const configName = fileName
      .replace('ws.', '')
      .replace(/-([a-z])/g, (_, letter) => letter.toUpperCase()) + '_config';

    // Build import path
    const importPath = `@/handlers/${modulePath ? modulePath + '/' : ''}${fileName}`;

    imports.push(`import { config as ${configName} } from '${importPath}';`);
    registryEntries.push(`  '${configName}': ${configName},`);

    console.log(`   ✓ Registered WebSocket handler: ${file}`);
  }

  const registryContent = `/**
 * WebSocket handler registry - Auto-generated
 * DO NOT EDIT - Generated by scripts/generate.ts
 */

import type { Context } from 'hono';
import type { App } from '@/types/app';
${imports.join('\n')}

export const wsRegistry = {
${registryEntries.join('\n')}
};

export function registerRoutes(app: App) {
  const ws = app.ws;

  for (const [name, config] of Object.entries(wsRegistry)) {
    const handlers = config.middleware || [];

    app.get(config.path, ...handlers, ws.upgradeWebSocket((ctx: Context) => ({
      async onOpen(event: MessageEvent, wsCtx: any) {
        const logger = ctx.get('logger');
        try {
          // Assign unique ID to raw WebSocket for reliable identity tracking across lifecycle methods
          // (Hono creates new WSContext wrappers for each lifecycle method, so we store on .raw which persists)
          const uniqueId = Math.random().toString(36).substring(7) + Date.now();
          (wsCtx.raw as any).__wsId = uniqueId;

          // Call handler's onConnect
          if (config.onConnect) {
            await config.onConnect(ctx, wsCtx);
          }
        } catch (error) {
          logger.error({
            error,
            handler: name,
            path: config.path,
            lifecycle: 'onConnect'
          }, 'WebSocket connection error');
          wsCtx.close(1011, 'Internal server error');
        }
      },

      async onMessage(event: MessageEvent, wsCtx: any) {
        const logger = ctx.get('logger');
        if (!config.onMessage) return;

        try {
          const message = JSON.parse(event.data.toString());
          await config.onMessage(ctx, wsCtx, message);
        } catch (error) {
          if (error instanceof SyntaxError) {
            logger.error({
              error,
              handler: name,
              path: config.path,
              rawData: event.data.toString()
            }, 'Failed to parse WebSocket message');
          } else {
            logger.error({
              error,
              handler: name,
              path: config.path,
              lifecycle: 'onMessage'
            }, 'WebSocket message processing error');
          }
        }
      },

      async onClose(event: CloseEvent, wsCtx: any) {
        const logger = ctx.get('logger');
        try {
          // Call handler's onClose
          if (config.onClose) {
            await config.onClose(ctx, wsCtx);
          }
        } catch (error) {
          logger.error({
            error,
            handler: name,
            path: config.path,
            lifecycle: 'onClose'
          }, 'WebSocket close error');
        }
      },

      async onError(event: Event, wsCtx: any) {
        const logger = ctx.get('logger');

        try {
          if (config.onError) {
            await config.onError(ctx, wsCtx, event);
          } else {
            // Default error handler - logs error with handler context
            logger.error({
              error: event,
              handler: name,
              path: config.path
            }, 'WebSocket error');
          }
        } catch (error) {
          logger.error({
            error,
            handler: name,
            path: config.path,
            lifecycle: 'onError'
          }, 'Error in WebSocket error handler');
        }
      },
    })));

    app.logger.debug(\`Registered WebSocket route: \${config.path}\`);
  }
}
`;

  await writeFile(path.join(WS_GENERATED_DIR, 'registry.ts'), registryContent);
  console.log('   ✓ Generated WebSocket registry\n');
}

// Run the script
main().catch(console.error);
