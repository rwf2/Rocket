#! /bin/bash

CA_SUBJECT="/C=US/ST=CA/O=Rocket CA/CN=Rocket Root CA"

openssl genrsa -out ca_key.pem 4096
openssl req -new -x509 -days 3650 -key ca_key.pem -subj "${CA_SUBJECT}" -out ca_cert.pem
