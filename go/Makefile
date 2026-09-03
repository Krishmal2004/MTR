.PHONY: dev build test lint sqlc

dev:
	go run ./cmd/server

build:
	go build -o server ./cmd/server

test:
	go test -v ./...

lint: 
	go vet ./...
	golangci-lint run

sqlc:
	cd internal/db && sqlc generate -f sqlc.yaml