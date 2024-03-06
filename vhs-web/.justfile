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

# Run a local dev server. Needs the vhs-server dev server running
dev:
    pnpm vite

build: check
    pnpm tsx build.ts

# Run a local server from the latest build artifacts. Needs the vhs-server dev server running
preview:
    pnpm vite preview
