.PHONY: test lint build vet fmt tidy doc clean

# Run all tests with verbose output
test:
	go test -v -count=1 -race ./...

# Run tests with coverage
cover:
	go test -coverprofile=coverage.out -race ./...
	go tool cover -html=coverage.out -o coverage.html
	@echo "Coverage report: coverage.html"

# Run linter (requires golangci-lint)
lint:
	golangci-lint run ./...

# Go vet
vet:
	go vet ./...

# Format code
fmt:
	gofmt -w .
	goimports -w .

# Tidy modules
tidy:
	go mod tidy

# Build (compile check)
build:
	go build ./...

# Generate godoc locally
doc:
	@echo "Open http://localhost:6060/pkg/github.com/chingchi/tslink-gosdk/"
	godoc -http=:6060

# Clean build artifacts
clean:
	rm -f coverage.out coverage.html

# CI: all checks
ci: tidy vet lint test
	@echo "All CI checks passed"
