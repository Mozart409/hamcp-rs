# https://just.systems

set dotenv-load

default:
    just --choose
    
inspector: clear
    npx @modelcontextprotocol/inspector --transport http --server-url http://localhost:3000/mcp

clear:
    clear

docker-build:
    docker build -t hamcp:local .

docker-run: docker-build
    docker run --rm -it \
        -p 3000:3000 \
        --env-file .env \
        hamcp:local

dcu: clear
    docker compose up --build -d --remove-orphans

dcd: clear
    docker compose down

trivy: clear docker-build
    trivy image hamcp:local

