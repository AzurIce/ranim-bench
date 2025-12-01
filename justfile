set windows-shell := ["powershell.exe", "-Command"]

graph:
    cargo run -- graph

[working-directory: 'web']
web: graph
    bun run build
