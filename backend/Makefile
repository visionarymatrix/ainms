.PHONY: dev build test lint migrate-up migrate-down fmt vet

# Build
build:
	go build -o bin/server ./cmd/server

# Run development server
dev: build
	./bin/server

# Run tests
test:
	go test -v -race ./...

# Run tests with coverage
test-cover:
	go test -v -race -coverprofile=coverage.out ./...
	go tool cover -html=coverage.out -o coverage.html

# Lint
lint:
	golangci-lint run ./...

# Format
fmt:
	gofmt -w .
	goimports -w .

# Vet
vet:
	go vet ./...

# Tidy
tidy:
	go mod tidy

# Database migrations
migrate-up:
	go run ./scripts/migrate.go up

migrate-down:
	go run ./scripts/migrate.go down

# Docker
docker-up:
	docker compose -f ../docker-compose.yml up -d

docker-down:
	docker compose -f ../docker-compose.yml down

docker-ps:
	docker compose -f ../docker-compose.yml ps

# Clean
clean:
	rm -rf bin/