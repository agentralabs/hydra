.PHONY: build run install clean test

# Build the binary
build:
	go build -o hydra-tui .

# Run in development
run:
	go run .

# Install globally
install:
	go install .

# Clean build artifacts
clean:
	rm -f hydra-tui

# Run tests
test:
	go test ./...

# Build for all platforms
release:
	GOOS=darwin GOARCH=arm64 go build -o hydra-tui-darwin-arm64 .
	GOOS=darwin GOARCH=amd64 go build -o hydra-tui-darwin-amd64 .
	GOOS=linux GOARCH=amd64 go build -o hydra-tui-linux-amd64 .
	GOOS=linux GOARCH=arm64 go build -o hydra-tui-linux-arm64 .
