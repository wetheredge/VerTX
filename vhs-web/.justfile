_default:
    @just --list --unsorted

clean:
    rm -r dist

fmt:
    pnpm biome format --write .
    cargo bin dprint fmt

check:
    pnpm biome ci .
    pnpm tsc

fix *args='.':
    pnpm biome lint --apply {{ args }}

# Run a local dev server. Needs the vhs-server dev server running
dev:
    pnpm vite

build: check
    NODE_ENV=production pnpm tsx build.ts

# Run a local server from the latest build artifacts. Needs the vhs-server dev server running
preview:
    pnpm vite preview
