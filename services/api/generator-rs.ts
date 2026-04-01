/**
 * generator-rs.ts — OpenAPI to Rust Code Generator
 *
 * Reads the OpenAPI spec and generates:
 * - src/generated/types.rs   — Rust structs for all request/response schemas
 * - src/generated/enums.rs   — Rust enums for all string enums
 * - src/generated/routes.rs  — Documented route list (comment block)
 * - src/generated/mod.rs     — Module declarations
 *
 * Usage: bun generator-rs.ts
 */

import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// ---------------------------------------------------------------------------
// Spec loading
// ---------------------------------------------------------------------------

const SPEC_CANDIDATES = [
  // TypeSpec-built spec (run `cd specs/api && bun run build` first)
  path.resolve(__dirname, '../../specs/api/dist/openapi/openapi.json'),
];

function loadSpec(): { spec: any; specPath: string } {
  for (const candidate of SPEC_CANDIDATES) {
    if (fs.existsSync(candidate)) {
      console.log(`Loading spec from: ${candidate}`);
      const spec = JSON.parse(fs.readFileSync(candidate, 'utf8'));
      return { spec, specPath: candidate };
    }
  }
  console.error('Error: No OpenAPI spec found. Tried:');
  for (const c of SPEC_CANDIDATES) console.error(`  ${c}`);
  console.error('\nRun one of:');
  console.error('  cd ../../specs/api && bun run build');
  console.error('  cd ../api && bun run generate');
  process.exit(1);
}

const { spec, specPath } = loadSpec();
const schemas: Record<string, any> = spec.components?.schemas ?? {};
const paths: Record<string, any> = spec.paths ?? {};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function toPascalCase(str: string): string {
  return str
    .replace(/[-_\s]+(.)?/g, (_, c) => (c ? c.toUpperCase() : ''))
    .replace(/^(.)/, (_, c) => c.toUpperCase());
}

function toSnakeCase(str: string): string {
  return str
    // Insert underscore before runs of uppercase letters followed by lowercase
    // e.g. "credentialID" -> "credential_ID", "credentialIDFoo" -> "credential_ID_Foo"
    .replace(/([a-z\d])([A-Z]+)([A-Z][a-z])/g, '$1_$2_$3')
    // Insert underscore before a single uppercase letter following a lowercase or digit
    .replace(/([a-z\d])([A-Z])/g, '$1_$2')
    .replace(/[-\s]+/g, '_')
    .toLowerCase()
    .replace(/^_/, '')
    .replace(/__+/g, '_');
}

/**
 * Resolve a $ref string like "#/components/schemas/Foo" to its schema object.
 */
function resolveRef(ref: string): { name: string; schema: any } | null {
  const parts = ref.replace(/^#\//, '').split('/');
  let node: any = spec;
  for (const part of parts) {
    node = node?.[part];
    if (!node) return null;
  }
  const name = parts[parts.length - 1];
  return { name, schema: node };
}

/**
 * Map an OpenAPI schema to a Rust type string.
 * nullable: if true, wraps the result in Option<T>.
 */
function schemaToRustType(schema: any, required: boolean = true): string {
  if (!schema) return 'serde_json::Value';

  // Handle $ref
  if (schema.$ref) {
    const resolved = resolveRef(schema.$ref);
    if (!resolved) return 'serde_json::Value';
    const rustName = toPascalCase(resolved.name);
    return required ? rustName : `Option<${rustName}>`;
  }

  // Handle anyOf / oneOf / allOf — use Value as a safe fallback
  if (schema.anyOf || schema.oneOf) {
    const variants: any[] = schema.anyOf ?? schema.oneOf;
    // If it's a simple nullable (one variant + null type), unwrap it
    const nonNull = variants.filter((v: any) => v.type !== 'null');
    if (nonNull.length === 1) {
      const inner = schemaToRustType(nonNull[0], true);
      return required ? `Option<${inner}>` : `Option<${inner}>`;
    }
    return required ? 'serde_json::Value' : 'Option<serde_json::Value>';
  }

  if (schema.allOf) {
    // Merge is complex — use Value
    return required ? 'serde_json::Value' : 'Option<serde_json::Value>';
  }

  const nullable = schema.nullable === true;

  let rustType: string;

  switch (schema.type) {
    case 'string': {
      if (schema.format === 'date-time') {
        rustType = 'chrono::DateTime<chrono::Utc>';
      } else if (schema.format === 'date') {
        rustType = 'chrono::NaiveDate';
      } else if (schema.format === 'uuid') {
        rustType = 'uuid::Uuid';
      } else if (schema.enum) {
        // Inline enum — use String, the named enum will be in enums.rs
        rustType = 'String';
      } else {
        rustType = 'String';
      }
      break;
    }
    case 'integer': {
      rustType = schema.format === 'int32' ? 'i32' : 'i64';
      break;
    }
    case 'number': {
      rustType = schema.format === 'float' ? 'f32' : 'f64';
      break;
    }
    case 'boolean': {
      rustType = 'bool';
      break;
    }
    case 'array': {
      const itemType = schema.items
        ? schemaToRustType(schema.items, true)
        : 'serde_json::Value';
      rustType = `Vec<${itemType}>`;
      break;
    }
    case 'object': {
      if (schema.additionalProperties && typeof schema.additionalProperties === 'object') {
        const valType = schemaToRustType(schema.additionalProperties, true);
        rustType = `std::collections::HashMap<String, ${valType}>`;
      } else {
        rustType = 'serde_json::Value';
      }
      break;
    }
    default: {
      rustType = 'serde_json::Value';
    }
  }

  if (!required || nullable) {
    return `Option<${rustType}>`;
  }
  return rustType;
}

/**
 * Determine whether a schema looks like an enum (has string enum values).
 */
function isStringEnum(schema: any): boolean {
  return (
    schema.type === 'string' &&
    Array.isArray(schema.enum) &&
    schema.enum.length > 0
  );
}

/**
 * Collect all top-level schemas that are string enums.
 */
function collectEnumSchemas(): Array<{ name: string; schema: any }> {
  const enums: Array<{ name: string; schema: any }> = [];
  for (const [name, schema] of Object.entries(schemas)) {
    if (isStringEnum(schema)) {
      enums.push({ name, schema });
    }
  }
  return enums;
}

/**
 * Collect all top-level schemas that are objects (struct candidates).
 * Skips pure enums.
 */
function collectStructSchemas(): Array<{ name: string; schema: any }> {
  const structs: Array<{ name: string; schema: any }> = [];
  for (const [name, schema] of Object.entries(schemas)) {
    if (isStringEnum(schema)) continue;

    const isObject =
      schema.type === 'object' ||
      schema.properties ||
      schema.allOf ||
      schema.anyOf;

    if (isObject) {
      structs.push({ name, schema });
    }
  }
  return structs;
}

// ---------------------------------------------------------------------------
// Enum generation
// ---------------------------------------------------------------------------

/**
 * Determine the best serde rename_all for an enum's variants.
 * Inspects whether values look like snake_case, kebab-case, or camelCase.
 */
function detectEnumRenameAll(values: string[]): string {
  const hasKebab = values.some(v => v.includes('-'));
  const hasSnake = values.some(v => v.includes('_'));
  const hasCamel = values.some(v => /[a-z][A-Z]/.test(v));

  if (hasKebab) return 'kebab-case';
  if (hasSnake) return 'snake_case';
  if (hasCamel) return 'camelCase';
  return 'snake_case';
}

/**
 * Convert a raw enum value string to a valid Rust identifier for a variant.
 * e.g. "non-binary" -> "NonBinary", "prefer_not_to_say" -> "PreferNotToSay"
 */
function enumValueToVariant(value: string): string {
  return value
    .replace(/[-_\s]+(.)?/g, (_, c) => (c ? c.toUpperCase() : ''))
    .replace(/^(.)/, (_, c) => c.toUpperCase())
    .replace(/[^a-zA-Z0-9]/g, '');
}

function generateEnums(): string {
  const enumSchemas = collectEnumSchemas();

  let out = `// AUTO-GENERATED by generator-rs.ts — DO NOT EDIT\n`;
  out += `// Source: ${specPath}\n`;
  out += `#![allow(dead_code)]\n\n`;
  out += `use serde::{Deserialize, Serialize};\n\n`;

  if (enumSchemas.length === 0) {
    out += `// No string enums found in spec.\n`;
    return out;
  }

  for (const { name, schema } of enumSchemas) {
    const rustName = toPascalCase(name);
    const values: string[] = schema.enum;
    const renameAll = detectEnumRenameAll(values);
    const doc = schema.description ?? schema.title ?? name;

    out += `/// ${doc}\n`;
    out += `#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]\n`;
    out += `#[serde(rename_all = "${renameAll}")]\n`;
    out += `pub enum ${rustName} {\n`;

    const seenVariants = new Set<string>();
    for (const value of values) {
      let variantName = enumValueToVariant(value);
      // Guard against duplicate variant names
      if (seenVariants.has(variantName)) {
        variantName = `${variantName}_${seenVariants.size}`;
      }
      seenVariants.add(variantName);

      // Only emit #[serde(rename = ...)] when the derived rename_all wouldn't
      // produce the correct wire value automatically. For simplicity we always
      // emit it so the mapping is explicit and rename_all can be removed safely.
      out += `    #[serde(rename = "${value}")]\n`;
      out += `    ${variantName},\n`;
    }

    out += `}\n\n`;
  }

  return out;
}

// ---------------------------------------------------------------------------
// Struct / types generation
// ---------------------------------------------------------------------------

/**
 * Merge properties from allOf schemas (simple merge, no circular detection).
 */
function mergeAllOf(schema: any): { properties: any; required: string[] } {
  const merged: any = {};
  const requiredFields: string[] = [];

  for (const sub of schema.allOf ?? []) {
    const resolved = sub.$ref ? resolveRef(sub.$ref)?.schema : sub;
    if (!resolved) continue;

    const props = resolved.properties ?? {};
    for (const [k, v] of Object.entries(props)) {
      merged[k] = v;
    }
    if (Array.isArray(resolved.required)) {
      requiredFields.push(...resolved.required);
    }
  }

  // Also include properties directly on the schema
  const direct = schema.properties ?? {};
  for (const [k, v] of Object.entries(direct)) {
    merged[k] = v;
  }
  if (Array.isArray(schema.required)) {
    requiredFields.push(...schema.required);
  }

  return { properties: merged, required: requiredFields };
}

function generateTypes(): string {
  const structSchemas = collectStructSchemas();
  const enumNames = new Set(collectEnumSchemas().map(e => toPascalCase(e.name)));

  let out = `// AUTO-GENERATED by generator-rs.ts — DO NOT EDIT\n`;
  out += `// Source: ${specPath}\n`;
  out += `#![allow(dead_code, unused_imports)]\n\n`;
  out += `use serde::{Deserialize, Serialize};\n`;
  out += `use serde_json::Value;\n`;
  out += `use std::collections::HashMap;\n`;

  // Import enums module if we have enums (avoid unused import warning)
  const enumCount = collectEnumSchemas().length;
  if (enumCount > 0) {
    out += `#[allow(unused_imports)]\n`;
    out += `use super::enums::*;\n`;
  }

  out += `\n`;

  if (structSchemas.length === 0) {
    out += `// No object schemas found in spec.\n`;
    return out;
  }

  // Track generated struct names to avoid duplicates
  const generated = new Set<string>();

  for (const { name, schema } of structSchemas) {
    const rustName = toPascalCase(name);
    if (generated.has(rustName)) continue;
    generated.add(rustName);

    // Resolve properties — handle allOf merging
    let properties: Record<string, any>;
    let requiredFields: string[];

    if (schema.allOf) {
      const merged = mergeAllOf(schema);
      properties = merged.properties;
      requiredFields = merged.required;
    } else {
      properties = schema.properties ?? {};
      requiredFields = schema.required ?? [];
    }

    const requiredSet = new Set(requiredFields);
    const doc = schema.description ?? schema.title ?? name;

    out += `/// ${doc}\n`;
    out += `#[derive(Debug, Clone, Serialize, Deserialize)]\n`;
    out += `#[serde(rename_all = "camelCase")]\n`;
    out += `pub struct ${rustName} {\n`;

    const propEntries = Object.entries(properties);

    if (propEntries.length === 0) {
      // Empty struct — add a catch-all field for forward compatibility
      out += `    #[serde(flatten)]\n`;
      out += `    pub extra: HashMap<String, Value>,\n`;
    } else {
      for (const [propName, propSchema] of propEntries) {
        const isRequired = requiredSet.has(propName);
        const propDoc = (propSchema as any).description ?? propName;

        // Check if this field's type is a named enum in our spec
        let fieldType: string;
        if ((propSchema as any).$ref) {
          const refName = (propSchema as any).$ref.split('/').pop();
          const refRustName = toPascalCase(refName);
          if (enumNames.has(refRustName)) {
            fieldType = isRequired ? refRustName : `Option<${refRustName}>`;
          } else {
            fieldType = schemaToRustType(propSchema as any, isRequired);
          }
        } else if (
          (propSchema as any).type === 'string' &&
          Array.isArray((propSchema as any).enum)
        ) {
          // Inline enum — use String for now
          fieldType = isRequired ? 'String' : 'Option<String>';
        } else {
          fieldType = schemaToRustType(propSchema as any, isRequired);
        }

        const snakeName = toSnakeCase(propName);
        const needsRename = snakeName !== propName;

        out += `    /// ${propDoc}\n`;

        // Skip rename if serde rename_all = camelCase handles it
        // We still emit it when the snake_case roundtrip would differ
        if (!isRequired) {
          out += `    #[serde(skip_serializing_if = "Option::is_none")]\n`;
        }

        out += `    pub ${snakeName}: ${fieldType},\n`;
      }
    }

    out += `}\n\n`;
  }

  return out;
}

// ---------------------------------------------------------------------------
// Routes generation
// ---------------------------------------------------------------------------

interface RouteInfo {
  method: string;
  path: string;
  operationId: string;
  summary: string;
  tags: string[];
  hasBody: boolean;
  hasQuery: boolean;
  hasPathParams: boolean;
}

function extractRoutes(): RouteInfo[] {
  const routes: RouteInfo[] = [];

  for (const [pathStr, pathItem] of Object.entries(paths)) {
    for (const [method, opSpec] of Object.entries(pathItem as any)) {
      if (!['get', 'post', 'patch', 'put', 'delete'].includes(method)) continue;
      const op = opSpec as any;
      if (!op.operationId) continue;

      const pathParams = (pathStr.match(/\{(\w+)\}/g) ?? []).map((p: string) =>
        p.replace(/[{}]/g, '')
      );
      const hasQuery = (op.parameters ?? []).some((p: any) => p.in === 'query');
      const hasBody =
        ['post', 'patch', 'put'].includes(method) &&
        !!(op.requestBody?.content?.['application/json']);

      routes.push({
        method: method.toUpperCase(),
        path: pathStr,
        operationId: op.operationId,
        summary: op.summary ?? op.description ?? op.operationId,
        tags: op.tags ?? [],
        hasBody,
        hasQuery,
        hasPathParams: pathParams.length > 0,
      });
    }
  }

  // Sort by tag then path for readability
  return routes.sort((a, b) => {
    const tagA = a.tags[0] ?? '';
    const tagB = b.tags[0] ?? '';
    if (tagA !== tagB) return tagA.localeCompare(tagB);
    return a.path.localeCompare(b.path);
  });
}

function generateRoutes(): string {
  const routes = extractRoutes();

  let out = `// AUTO-GENERATED by generator-rs.ts — DO NOT EDIT\n`;
  out += `// Source: ${specPath}\n`;
  out += `//\n`;
  out += `// This file documents all API routes from the OpenAPI spec.\n`;
  out += `// Actual routing is wired manually in src/handlers/.\n`;
  out += `// Re-run the generator after updating the spec to refresh this list.\n`;
  out += `#![allow(dead_code)]\n\n`;

  if (routes.length === 0) {
    out += `// No routes found in spec.\n`;
    return out;
  }

  // Group by tag
  const byTag = new Map<string, RouteInfo[]>();
  for (const route of routes) {
    const tag = route.tags[0] ?? 'Untagged';
    if (!byTag.has(tag)) byTag.set(tag, []);
    byTag.get(tag)!.push(route);
  }

  // Emit a constant listing the route count
  out += `/// Total number of routes in the OpenAPI spec.\n`;
  out += `pub const ROUTE_COUNT: usize = ${routes.length};\n\n`;

  // Emit a struct representing a single route entry
  out += `/// Static route metadata entry.\n`;
  out += `#[derive(Debug)]\n`;
  out += `pub struct RouteEntry {\n`;
  out += `    pub method: &'static str,\n`;
  out += `    pub path: &'static str,\n`;
  out += `    pub operation_id: &'static str,\n`;
  out += `    pub summary: &'static str,\n`;
  out += `    pub tag: &'static str,\n`;
  out += `    pub has_body: bool,\n`;
  out += `    pub has_query: bool,\n`;
  out += `    pub has_path_params: bool,\n`;
  out += `}\n\n`;

  // Emit the full route table as a static array
  out += `/// All API routes from the OpenAPI spec.\n`;
  out += `pub static ALL_ROUTES: &[RouteEntry] = &[\n`;

  for (const route of routes) {
    const tag = route.tags[0] ?? 'Untagged';
    // Escape backslashes and double-quotes for Rust string literals
    const escapedSummary = route.summary.replace(/\\/g, '\\\\').replace(/"/g, '\\"');
    out += `    RouteEntry {\n`;
    out += `        method: "${route.method}",\n`;
    out += `        path: "${route.path}",\n`;
    out += `        operation_id: "${route.operationId}",\n`;
    out += `        summary: "${escapedSummary}",\n`;
    out += `        tag: "${tag}",\n`;
    out += `        has_body: ${route.hasBody},\n`;
    out += `        has_query: ${route.hasQuery},\n`;
    out += `        has_path_params: ${route.hasPathParams},\n`;
    out += `    },\n`;
  }

  out += `];\n\n`;

  // Emit a human-readable comment listing per tag
  out += `/*\n`;
  out += ` * ROUTE SUMMARY\n`;
  out += ` * ==============\n`;
  out += ` * Total: ${routes.length} routes\n`;
  out += ` *\n`;

  for (const [tag, tagRoutes] of byTag) {
    out += ` * [${tag}] (${tagRoutes.length} routes)\n`;
    for (const r of tagRoutes) {
      const flags: string[] = [];
      if (r.hasBody) flags.push('body');
      if (r.hasQuery) flags.push('query');
      if (r.hasPathParams) flags.push('path-params');
      const flagStr = flags.length > 0 ? `  [${flags.join(', ')}]` : '';
      out += ` *   ${r.method.padEnd(7)} ${r.path}${flagStr}\n`;
      out += ` *            operationId: ${r.operationId}\n`;
    }
    out += ` *\n`;
  }

  out += ` */\n`;

  return out;
}

// ---------------------------------------------------------------------------
// mod.rs generation
// ---------------------------------------------------------------------------

function generateMod(): string {
  let out = `// AUTO-GENERATED by generator-rs.ts — DO NOT EDIT\n`;
  out += `// Source: ${specPath}\n\n`;
  out += `pub mod enums;\n`;
  out += `pub mod types;\n`;
  out += `pub mod routes;\n`;
  return out;
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

const OUTPUT_DIR = path.join(__dirname, 'src/generated');

if (!fs.existsSync(OUTPUT_DIR)) {
  fs.mkdirSync(OUTPUT_DIR, { recursive: true });
}

const structCount = collectStructSchemas().length;
const enumCount = collectEnumSchemas().length;
const routes = extractRoutes();

console.log(`Spec summary:`);
console.log(`  Schemas : ${Object.keys(schemas).length} (${structCount} structs, ${enumCount} enums)`);
console.log(`  Routes  : ${routes.length} across ${Object.keys(paths).length} paths`);
console.log('');

fs.writeFileSync(path.join(OUTPUT_DIR, 'enums.rs'), generateEnums());
console.log(`  enums.rs  — ${enumCount} enum(s)`);

fs.writeFileSync(path.join(OUTPUT_DIR, 'types.rs'), generateTypes());
console.log(`  types.rs  — ${structCount} struct(s)`);

fs.writeFileSync(path.join(OUTPUT_DIR, 'routes.rs'), generateRoutes());
console.log(`  routes.rs — ${routes.length} route(s)`);

fs.writeFileSync(path.join(OUTPUT_DIR, 'mod.rs'), generateMod());
console.log(`  mod.rs    — module declarations`);

console.log('');
console.log(`Done. Generated files in: ${OUTPUT_DIR}`);
