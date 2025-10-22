import type { JsonRpcRequest, JsonRpcResponse } from '../types/protocol'
import type { InitializeParams, InitializeResponse } from '../../bindings'

export class JsonRpcError extends Error {
  constructor(
    message: string,
    public code: number,
    public data?: unknown
  ) {
    super(message)
    this.name = 'JsonRpcError'
  }
}

/**
 * JSON-RPC 2.0 client for communicating with the Codex app server
 */
export class JsonRpcClient {
  private requestId = 0
  private isInitialized = false

  constructor(private baseUrl: string = '/api') {}

  /**
   * Initialize the connection with the app server
   */
  async initialize(clientInfo: InitializeParams['clientInfo']): Promise<InitializeResponse> {
    const response = await this.call<InitializeParams, InitializeResponse>(
      'initialize',
      { clientInfo }
    )
    this.isInitialized = true
    return response
  }

  /**
   * Check if the client is initialized
   */
  getIsInitialized(): boolean {
    return this.isInitialized
  }

  /**
   * Make a JSON-RPC call to the server
   */
  async call<TParams, TResponse>(
    method: string,
    params: TParams
  ): Promise<TResponse> {
    const id = ++this.requestId

    const request: JsonRpcRequest<TParams> = {
      jsonrpc: '2.0',
      method,
      params,
      id,
    }

    const response = await fetch(this.baseUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(request, (_key, value) =>
        typeof value === 'bigint' ? value.toString() : value
      ),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    const text = await response.text()
    const json: JsonRpcResponse<TResponse> = JSON.parse(text, (_key, value) => {
      // Convert numeric strings that end with 'n' or are too large for Number back to bigint
      if (typeof value === 'string' && /^-?\d+$/.test(value)) {
        const num = Number(value)
        if (!Number.isSafeInteger(num)) {
          return BigInt(value)
        }
      }
      return value
    })

    if (json.error) {
      throw new JsonRpcError(json.error.message, json.error.code, json.error.data)
    }

    if (json.result === undefined) {
      throw new Error('Invalid JSON-RPC response: missing result')
    }

    return json.result
  }
}

// Get base URL from environment or use default
const getBaseUrl = () => {
  // Use VITE_API_BASE_URL if set, otherwise default to /api
  return import.meta.env.VITE_API_BASE_URL || '/api';
};

// Singleton instance for the app
export const apiClient = new JsonRpcClient(getBaseUrl())
