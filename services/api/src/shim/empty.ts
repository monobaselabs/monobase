/**
 * Empty shim for modules that aren't available in Boa
 * Used by esbuild alias to replace unavailable node modules
 */
export default {};
export const randomBytes = () => { throw new Error('Not available in Boa'); };
export const createHash = () => { throw new Error('Not available in Boa'); };
export const createHmac = () => { throw new Error('Not available in Boa'); };
