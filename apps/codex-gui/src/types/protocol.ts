/**
 * Auto-generated TypeScript bindings from Rust protocol definitions.
 * Generated from codex-rs/app-server-protocol via codex-protocol-ts.
 * 
 * Re-exports all protocol types from the bindings directory.
 * Do not modify this file directly - regenerate bindings instead.
 */

// Re-export all generated protocol types
export * from '../../bindings'

// JSON-RPC types
export interface JsonRpcRequest<T = unknown> {
  jsonrpc: '2.0'
  method: string
  params: T
  id: string | number
}

export interface JsonRpcResponse<T = unknown> {
  jsonrpc: '2.0'
  result?: T
  error?: {
    code: number
    message: string
    data?: unknown
  }
  id: string | number
}
