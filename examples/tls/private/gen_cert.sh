#! /bin/bash

# TODO: `rustls` (really, `webpki`) doesn't currently use the CN in the subject
# to check if a certificate is valid for a server name sent via SNI. It's not
# clear if this is intended, since certificates _should_ have a `subjectAltName`
# with a DNS name, or if it simply hasn't been implemented yet. See
# https://bugzilla.mozilla.org/show_bug.cgi?id=552346 for a bit more info.

CA_SUBJECT="/C=US/ST=CA/O=Rocket CA/CN=Rocket Root CA"
SUBJECT="/C=US/ST=CA/O=Rocket/CN=localhost"
ALT="DNS:localhost"

openssl genrsa -out ca_key.pem 4096
openssl req -new -x509 -days 3650 -key ca_key.pem -subj "${CA_SUBJECT}" -out ca_cert.pem

openssl req -newkey rsa:4096 -nodes -sha256 -keyout key.pem -subj "${SUBJECT}" -out server.csr
openssl x509 -req -sha256 -extfile <(printf "subjectAltName=${ALT}") -days 3650 \
    -CA ca_cert.pem -CAkey ca_key.pem -CAcreateserial \
    -in server.csr -out cert.pem

rm ca_cert.srl server.csr
