#!/bin/bash

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

set -e

CERT_DIR="${1:-.}"
DAYS_VALID=365

echo -e "${YELLOW}🔐 Generating CA-signed certificates for localhost${NC}"
echo "Output directory: $CERT_DIR"
echo ""

# Create directory if it doesn't exist
mkdir -p "$CERT_DIR"

# Step 1: Generate CA private key
echo -e "${YELLOW}[1/6] Generating CA private key...${NC}"
openssl genrsa -out "$CERT_DIR/ca.key" 2048 2>/dev/null

# Step 2: Generate CA certificate (self-signed)
echo -e "${YELLOW}[2/6] Generating CA certificate...${NC}"
openssl req -new -x509 -days $DAYS_VALID -key "$CERT_DIR/ca.key" \
  -out "$CERT_DIR/ca.pem" \
  -subj "/C=US/ST=Development/L=Local/O=Quake Dev/CN=Quake Dev CA" \
  2>/dev/null

# Step 3: Generate server private key
echo -e "${YELLOW}[3/6] Generating server private key...${NC}"
openssl genrsa -out "$CERT_DIR/key.pem" 2048 2>/dev/null

# Step 4: Create certificate extensions file for SANs
echo -e "${YELLOW}[4/6] Creating certificate extensions...${NC}"
cat > "$CERT_DIR/cert.ext" << EOF
subjectAltName=DNS:localhost,DNS:127.0.0.1,IP:127.0.0.1,IP:0.0.0.0
keyUsage=digitalSignature,keyEncipherment
extendedKeyUsage=serverAuth,clientAuth
EOF

# Step 5: Generate certificate signing request
echo -e "${YELLOW}[5/6] Generating certificate signing request...${NC}"
openssl req -new -key "$CERT_DIR/key.pem" \
  -out "$CERT_DIR/server.csr" \
  -subj "/C=US/ST=Development/L=Local/O=Quake Dev/CN=localhost" \
  2>/dev/null

# Step 6: Sign the certificate with CA
echo -e "${YELLOW}[6/6] Signing server certificate with CA...${NC}"
openssl x509 -req -in "$CERT_DIR/server.csr" \
  -CA "$CERT_DIR/ca.pem" -CAkey "$CERT_DIR/ca.key" \
  -CAcreateserial -out "$CERT_DIR/cert.pem" \
  -days $DAYS_VALID \
  -extfile "$CERT_DIR/cert.ext" \
  2>/dev/null

# Cleanup temporary files
rm -f "$CERT_DIR/server.csr" "$CERT_DIR/cert.ext" "$CERT_DIR/ca.srl"

echo ""
echo -e "${GREEN}✅ Certificates generated successfully!${NC}"
echo ""
echo "Generated files:"
echo "  - $CERT_DIR/ca.pem       (CA certificate)"
echo "  - $CERT_DIR/cert.pem     (Server certificate - signed by CA)"
echo "  - $CERT_DIR/key.pem      (Server private key)"
echo "  - $CERT_DIR/ca.key       (CA private key - keep secure!)"
echo ""
echo "Certificate details:"
openssl x509 -in "$CERT_DIR/cert.pem" -text -noout | grep -E "Subject:|Issuer:|Not Before|Not After|DNS:|IP Address:" | sed 's/^/  /'
echo ""
echo "Certificate verification:"
if openssl verify -CAfile "$CERT_DIR/ca.pem" "$CERT_DIR/cert.pem" > /dev/null 2>&1; then
  echo -e "${GREEN}  ✓ Certificate is signed by CA${NC}"
else
  echo -e "${RED}  ✗ Certificate verification failed${NC}"
fi
echo ""
echo -e "${GREEN}✨ Done!${NC}"
