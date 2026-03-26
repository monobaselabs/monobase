import { getTransport, setHttpBaseUrl, setTransportMode } from './transport'
export { setTransportMode, getTransportInfo, isTauriEnvironment, isEmbeddedMode } from './transport'

/**
 * API Error class for handling API errors consistently
 */
export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
    public data?: any
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

/**
 * Generic paginated response matching TypeSpec OffsetPaginatedResponse
 * Used for list endpoints that return data with pagination metadata
 */
export interface PaginatedResponse<T> {
  data: T[]
  pagination: {
    offset: number
    limit: number
    count: number
    totalCount: number
    totalPages: number
    currentPage: number
    hasNextPage: boolean
    hasPreviousPage: boolean
  }
}

// Global API base URL - set by ApiProvider or manually
let globalApiBaseUrl = 'http://localhost:7213'

/**
 * Set the global API base URL
 */
export function setApiBaseUrl(url: string) {
  globalApiBaseUrl = url
  setHttpBaseUrl(url) // Also update transport layer
}

/**
 * Get the current API base URL
 */
export function getApiBaseUrl(): string {
  return globalApiBaseUrl
}

/**
 * API request using transport abstraction
 * Automatically switches between HTTP and IPC based on environment
 */
async function api<T = any>(
  url: string,
  options?: RequestInit
): Promise<T> {
  const transport = getTransport()

  // Extract headers from options
  const headers: Record<string, string> = {}
  if (options?.headers) {
    const optHeaders = options.headers
    if (optHeaders instanceof Headers) {
      optHeaders.forEach((value, key) => { headers[key] = value })
    } else if (Array.isArray(optHeaders)) {
      for (const [key, value] of optHeaders) {
        headers[key] = value
      }
    } else {
      Object.assign(headers, optHeaders)
    }
  }

  try {
    const response = await transport.request({
      method: options?.method || 'GET',
      url,
      body: options?.body as string | undefined,
      headers,
    })

    // Handle response
    if (response.status >= 400) {
      let errorData
      try {
        errorData = JSON.parse(response.body)
      } catch {
        errorData = { message: `API Error: ${response.status}` }
      }

      throw new ApiError(
        response.status,
        errorData.message || `API Error: ${response.status}`,
        errorData
      )
    }

    // Handle no content responses
    if (response.status === 204 || !response.body) {
      return {} as T
    }

    // Parse JSON response
    try {
      return JSON.parse(response.body)
    } catch {
      return {} as T
    }
  } catch (error) {
    // Re-throw ApiError as-is
    if (error instanceof ApiError) {
      throw error
    }

    // Handle timeout errors
    if (error instanceof Error && error.name === 'AbortError') {
      throw new ApiError(
        408,
        'Request timeout - the server took too long to respond. Please check your connection and try again.',
        { timeout: true }
      )
    }

    // Re-throw other errors
    throw error
  }
}

/**
 * Convenience methods for common HTTP methods
 */
export const apiGet = <T = any>(url: string, params?: Record<string, any>) => {
  // Filter out undefined values to prevent URLSearchParams converting them to "undefined" strings
  const cleanParams = params
    ? Object.fromEntries(
        Object.entries(params).filter(([_, value]) => value !== undefined)
      )
    : undefined

  const queryString = cleanParams ? `?${new URLSearchParams(cleanParams).toString()}` : ''
  return api<T>(`${url}${queryString}`, { method: 'GET' })
}

export const apiPost = <T = any>(url: string, data?: any) =>
  api<T>(url, { method: 'POST', body: data ? JSON.stringify(data) : undefined })

export const apiPatch = <T = any>(url: string, data?: any) =>
  api<T>(url, { method: 'PATCH', body: data ? JSON.stringify(data) : undefined })

export const apiDelete = <T = any>(url: string) =>
  api<T>(url, { method: 'DELETE' })
