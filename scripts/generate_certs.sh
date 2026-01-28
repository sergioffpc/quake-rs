
#!/bin/bash

# Generate self-signed certificates for Quinn QUIC connections
# Usage: ./generate_certs.sh [output_dir] [domain]

set -e

OUTPUT_DIR="${1:-./certs}"
DOMAIN="${2:-localhost}"
DAYS_VALID=365

# Create output directory
mkdir -p "$OUTPUT_DIR"

echo "Generating certificates in: $OUTPUT_DIR"
echo "Domain: $DOMAIN"
echo "Valid for: $DAYS_VALID days"
echo ""

# Generate CA private key
echo "==> Generating CA private key..."
openssl genrsa -out "$OUTPUT_DIR/ca.key" 4096

# Generate CA certificate
echo "==> Generating CA certificate..."
openssl req -x509 -new -nodes \
    -key "$OUTPUT_DIR/ca.key" \
    -sha256 \
    -days $DAYS_VALID \
    -out "$OUTPUT_DIR/ca.crt" \
    -subj "/C=US/ST=Local/L=Local/O=Quake-Dev/CN=Quake-CA"

# Generate server private key
echo "==> Generating server private key..."
openssl genrsa -out "$OUTPUT_DIR/server.key" 2048

# Generate server CSR
echo "==> Generating server CSR..."
openssl req -new \
    -key "$OUTPUT_DIR/server.key" \
    -out "$OUTPUT_DIR/server.csr" \
    -subj "/C=US/ST=Local/L=Local/O=Quake-Dev/CN=$DOMAIN"

# Create server extensions file (required for SAN)
cat > "$OUTPUT_DIR/server_ext.cnf" << EOF
authorityKeyIdentifier=keyid,issuer
basicConstraints=CA:FALSE
keyUsage = digitalSignature, nonRepudiation, keyEncipherment, dataEncipherment
subjectAltName = @alt_names

[alt_names]
DNS.1 = $DOMAIN
DNS.2 = *.$DOMAIN
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

# Sign server certificate with CA
echo "==> Signing server certificate..."
openssl x509 -req \
    -in "$OUTPUT_DIR/server.csr" \
    -CA "$OUTPUT_DIR/ca.crt" \
    -CAkey "$OUTPUT_DIR/ca.key" \
    -CAcreateserial \
    -out "$OUTPUT_DIR/server.crt" \
    -days $DAYS_VALID \
    -sha256 \
    -extfile "$OUTPUT_DIR/server_ext.cnf"

# Cleanup intermediate files
rm -f "$OUTPUT_DIR"/*.csr "$OUTPUT_DIR"/*.cnf "$OUTPUT_DIR"/*.srl

echo ""
echo "==> Certificates generated successfully!"
echo ""
echo "Files created:"
echo "  CA:     $OUTPUT_DIR/ca.crt, $OUTPUT_DIR/ca.key"
echo "  Server: $OUTPUT_DIR/server.crt, $OUTPUT_DIR/server.key"
echo ""
echo "Quake usage example (Rust):"
echo "  - Server: Load server.crt and server.key"
echo "  - Client: Load ca.crt (for server verification)"
