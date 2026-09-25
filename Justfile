
git_revision := `git rev-parse --short HEAD`
app_version := `awk -F'"' '/^\[package\]/{p=1} p && /^version *=/{print $2; exit}' Cargo.toml`
build_date := `date -u +%Y-%m-%dT%H:%M:%SZ`

image_quay   := 'quay.io/tama5'
image_github := 'ghcr.io/tamada'
container_image := image_quay

container_runner := "docker"

test:
    cargo llvm-cov

generate_demo: build
    PATH=target/release:$PATH vhs .github/assets/demo.tape

docs:
    cargo llvm-cov --html
    cd docs && {{ container_runner }} run -v $PWD:/project -it --rm ghcr.io/gohugoio/hugo:v0.165.0
    rm -rf docs/public/coverage && cp -r target/llvm-cov/html docs/public/coverage

build: test
    cargo build --release

container-local:
    {{ container_runner }} build \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{app_version}} \
        -t {{container_image}}/mosty:latest \
        -t {{container_image}}/mosty:{{ app_version }} \
        -f Containerfile \
        .

container:
    {{ container_runner }} buildx build --push \
        --platform linux/amd64,linux/arm64 \
        --build-arg GIT_REVISION={{git_revision}} \
        --build-arg BUILD_DATE={{build_date}} \
        --build-arg VERSION={{ app_version }} \
        -t {{container_image}}/mosty:latest \
        -t {{container_image}}/mosty:{{ app_version }} \
        -f Containerfile \
        .
