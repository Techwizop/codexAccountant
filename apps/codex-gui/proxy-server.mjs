import { spawn } from 'child_process'
import http from 'http'

const PORT = 8080

// Spawn the Rust app server
const appServer = spawn('cargo', ['run', '--features', 'ledger', '--bin', 'codex-app-server'], {
  cwd: '../../codex-rs',
  env: { ...process.env, CODEX_LEDGER_IN_MEMORY: '1', PATH: `${process.env.USERPROFILE}\\.cargo\\bin;${process.env.PATH}` },
  stdio: ['pipe', 'pipe', 'inherit']
})

let requestId = 0
const pendingRequests = new Map()

// Handle responses from app server
let buffer = ''
appServer.stdout.on('data', (data) => {
  buffer += data.toString()
  const lines = buffer.split('\n')
  buffer = lines.pop() || ''
  
  for (const line of lines) {
    if (!line.trim()) continue
    try {
      const response = JSON.parse(line)
      if (response.id !== undefined) {
        const pending = pendingRequests.get(response.id)
        if (pending) {
          pendingRequests.delete(response.id)
          pending.resolve(response)
        }
      }
    } catch (err) {
      console.error('Failed to parse response:', err, line)
    }
  }
})

appServer.on('error', (err) => {
  console.error('App server error:', err)
})

appServer.on('exit', (code) => {
  console.log(`App server exited with code ${code}`)
  process.exit(code || 0)
})

// HTTP server
const server = http.createServer(async (req, res) => {
  if (req.method === 'POST' && req.url === '/api') {
    let body = ''
    req.on('data', chunk => body += chunk)
    req.on('end', async () => {
      try {
        const request = JSON.parse(body)
        const id = request.id || ++requestId
        
        // Send request to app server
        const promise = new Promise((resolve, reject) => {
          const timeout = setTimeout(() => {
            pendingRequests.delete(id)
            reject(new Error('Request timeout'))
          }, 30000)
          
          pendingRequests.set(id, {
            resolve: (response) => {
              clearTimeout(timeout)
              resolve(response)
            },
            reject
          })
        })
        
        appServer.stdin.write(JSON.stringify({ ...request, id }) + '\n')
        
        const response = await promise
        res.writeHead(200, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify(response))
      } catch (err) {
        console.error('Request error:', err)
        res.writeHead(500, { 'Content-Type': 'application/json' })
        res.end(JSON.stringify({
          jsonrpc: '2.0',
          error: { code: -32603, message: err.message },
          id: null
        }))
      }
    })
  } else {
    res.writeHead(404)
    res.end()
  }
})

server.listen(PORT, () => {
  console.log(`Proxy server listening on http://localhost:${PORT}`)
  console.log('Proxying requests to codex-app-server via stdio')
})

process.on('SIGINT', () => {
  console.log('\nShutting down...')
  appServer.kill()
  server.close()
  process.exit(0)
})
