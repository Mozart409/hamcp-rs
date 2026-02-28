# https://just.systems

set dotenv-load

default:
    just --choose
    
inspector: clear
    npx @modelcontextprotocol/inspector http://localhost:3000/mcp

clear:
    clear
