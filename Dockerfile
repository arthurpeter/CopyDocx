# Use the official Rust image as the base image
FROM rust:latest AS builder

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the Cargo.toml, Cargo.lock, and .env files
COPY Cargo.toml Cargo.lock .env ./

# Copy the source code
COPY src ./src
COPY static ./static

# Build the project
RUN cargo build --release

# Use a more complete base image for the final stage
FROM ubuntu:22.04

# Install necessary dependencies
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

# Set the working directory inside the container
WORKDIR /usr/src/app

# Copy the built binary from the builder stage
COPY --from=builder /usr/src/app/target/release/copydocx .

# Copy static files
COPY --from=builder /usr/src/app/static /usr/src/app/static

# Copy the .env file
COPY --from=builder /usr/src/app/.env .env

# Expose the port the application runs on
EXPOSE 80

# Set the entrypoint to the built binary
CMD ["./copydocx"]