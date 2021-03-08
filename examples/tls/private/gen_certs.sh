#! /bin/bash

# Should use gen_ca.sh first to generate the CA.

# To generate certificates of specific private key type, pass any of the following arguements:
# 'ed25519', 'rsa_sha256', 'ecdsa_nistp256_sha256' or 'ecdsa_nistp384_sha384'
#
# If no argument is passed all supported certificates types will be generated.
#

# TODO: `rustls` (really, `webpki`) doesn't currently use the CN in the subject
# to check if a certificate is valid for a server name sent via SNI. It's not
# clear if this is intended, since certificates _should_ have a `subjectAltName`
# with a DNS name, or if it simply hasn't been implemented yet. See
# https://bugzilla.mozilla.org/show_bug.cgi?id=552346 for a bit more info.

SUBJECT="/C=US/ST=CA/O=Rocket/CN=localhost"
ALT="DNS:localhost"

function gen_rsa_sha256() {
    openssl req -newkey rsa:4096 -nodes -sha256 -keyout rsa_sha256_key.pem -subj "${SUBJECT}" -out server.csr
    openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
        -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
        -in server.csr -out rsa_sha256_cert.pem

    rm ca_cert.srl server.csr
}

function gen_ed25519() {
    openssl genpkey -algorithm ED25519 > ed25519_key.pem

    openssl req -new -key ed25519_key.pem -subj "${SUBJECT}" -out server.csr
    openssl x509 -req -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
        -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
        -in server.csr -out ed25519_cert.pem

    rm ca_cert.srl server.csr
}

function gen_ecdsa_nistp256_sha256() {
    openssl ecparam -out ecdsa_nistp256_sha256_key.pem -name prime256v1 -genkey

    # Convert to pkcs8 format supported by rustls
    openssl pkcs8 -topk8 -nocrypt -in ecdsa_nistp256_sha256_key.pem -out ecdsa_nistp256_sha256_key_pkcs8.pem

    openssl req -new -nodes -sha256 -key ecdsa_nistp256_sha256_key_pkcs8.pem -subj "${SUBJECT}" -out server.csr
    openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
        -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
        -in server.csr -out ecdsa_nistp256_sha256_cert.pem

    rm ca_cert.srl server.csr ecdsa_nistp256_sha256_key.pem
}


function gen_ecdsa_nistp384_sha384() {
    openssl ecparam -out ecdsa_nistp384_sha384_key.pem -name secp384r1 -genkey

    # Convert to pkcs8 format supported by rustls
    openssl pkcs8 -topk8 -nocrypt -in ecdsa_nistp384_sha384_key.pem -out ecdsa_nistp384_sha384_key_pkcs8.pem

    openssl req -new -nodes -sha384 -key ecdsa_nistp384_sha384_key_pkcs8.pem -subj "${SUBJECT}" -out server.csr
    openssl x509 -req -sha384 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
        -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
        -in server.csr -out ecdsa_nistp384_sha384_cert.pem

    rm ca_cert.srl server.csr ecdsa_nistp384_sha384_key.pem
}

case $1 in
  ed25519) gen_ed25519 ;;
  rsa_sha256) gen_rsa_sha256 ;;
  ecdsa_nistp256_sha256) gen_ecdsa_nistp256_sha256 ;;
  ecdsa_nistp384_sha384) gen_ecdsa_nistp384_sha384 ;;
  *)
    gen_ed25519
    gen_rsa_sha256
    gen_ecdsa_nistp256_sha256
    gen_ecdsa_nistp384_sha384
    ;;
esac
