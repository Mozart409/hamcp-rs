# https://just.systems

set dotenv-load

default:
    just --choose
    
inspector: clear
    npx @modelcontextprotocol/inspector

clear:
    clear
