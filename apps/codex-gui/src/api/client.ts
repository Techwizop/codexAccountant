import type { JsonRpcRequest, JsonRpcResponse } from '../types/protocol'

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

  constructor(private baseUrl: string = '/api') {}

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
      body: JSON.stringify(request),
    })

    if (!response.ok) {
      throw new Error(`HTTP ${response.status}: ${response.statusText}`)
    }

    const json: JsonRpcResponse<TResponse> = await response.json()

    if (json.error) {
      throw new JsonRpcError(json.error.message, json.error.code, json.error.data)
    }

    if (json.result === undefined) {
      throw new Error('Invalid JSON-RPC response: missing result')
    }

    return json.result
  }
}

// Singleton instance for the app
export const apiClient = new JsonRpcClient()
