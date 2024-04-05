_default:
    @just --list --unsorted

clean:
    rm -r dist

fmt:
    bun run -b biome format --write .
    cargo bin dprint fmt

check:
    bun run -b biome ci .
    bun run -b tsc

fix *args='.':
    bun run -b biome lint --apply {{ args }}

# Run a local dev server. Needs the vertx-api dev server running
dev:
    bun run -b vite

build: check
    NODE_ENV=production bun run -b vite build
    bun run compress.ts

# Run a local server from the latest build artifacts. Needs the vertx-api dev server running
preview:
    bun run -b vite preview
