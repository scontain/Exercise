# Confidential CoSign, confidential GPG, and confidential SCONE-Signer

This Section is all about signing container images protecting the key material and implementing access control with the
help of OTPs.

We assume the CLI is split in two parts. An *admin part* that is executed on a trusted host and the *worker part* which
is executed on an untrusted host, e.g., a cloud computer.

## Lessons

- transform a native Go application into a confidential Go application
- transform a native application (installed as a package) into a  confidential application
- use of host (aka native) and confidential arguments
- signing a confidential binary with Scone signer

We revisit
- creating a policy for cosign and  GPG that is protected with an OTP
- build a container for cosign using a Dockerfile
- storing credentials in an encrypted volume

We show how to

- sign container images using access control and encrypted credentials
- sign messages using GPG using access control and encrypted credentials

##  Assignments

These assignments are an extension of the assignments of the last section. 


## Task 1: Create a container image with a confidential cosign

- Cross-compile the container image using our slightly extended Google Go compiler. This results in binary that is linked against `glibc` instead of performing system calls directly.
- Transform the compiled native application into a confidential application using `scone-signer` application
- Create an image that contains the confidential application

## Task 2: Create a wrapper code for the signing and verifying container images

We use an unmodified cosign which we can use

```bash
COSIGN_PASSWORD=my_secret_password cosign generate-key-pair
```

to generate a key pair.

We can then use this key pair to sign container images:

```bash
scone cosign sign  --key cosign.key @1
```

We can also verify the signature of container images.

scone cosign verify --key cosign.key @1


```bash
scone cosign sign  --key cosign.key MyImage
```

We can also verify the signature of container images.

```bash
scone cosign verify --key cosign.key MyImage
```


## Next Steps

As we see, there is a lot of boilerplate code and we might need to reimplement the same steps for each new application like `GPG`. Hence, we show how to automate these steps in the next section.



