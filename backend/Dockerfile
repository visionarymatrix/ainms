# Stage 1: Build the Go binary
FROM golang:1.26-alpine AS builder
WORKDIR /app

# Install git and certificates
RUN apk add --no-cache git ca-certificates build-base

# Copy module files and download dependencies
COPY go.mod go.sum ./
RUN go mod download

# Copy the rest of the source code
COPY . .

# Build the binary
RUN CGO_ENABLED=0 GOOS=linux go build -o bin/server ./cmd/server

# Stage 2: Runtime image
FROM alpine:3.19
WORKDIR /app

# Install curl for healthchecks and ca-certificates
RUN apk add --no-cache curl ca-certificates

# Copy the binary from builder
COPY --from=builder /app/bin/server /app/server

# Create directory for uploads/screenshots
RUN mkdir -p public/screenshots

# Expose port
EXPOSE 8440

# Run the server
CMD ["/app/server"]
