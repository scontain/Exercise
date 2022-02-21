# Confidential CoSign, confidential GPG, and confidential SCONE-Signer

This Section is all about signing images, binaries, text and protecting the key material and having an additional access control.

## Lessons

- transform a native Go application into a confidential Go application
- transform a native application (installed as a package) into a  confidential application
- use of native and confidential arguments
- signing a confidential binary with Scone signer

We revisit

- creating a policy for cosign and  GPG that is protected with an OTP
- build a container for cosign using a Dockerfile
- storing credentials in an encrypted volume

We show how to

- sign container images using access control and encrypted credentials
- sign messages using GPG using access control and encrypted credentials

