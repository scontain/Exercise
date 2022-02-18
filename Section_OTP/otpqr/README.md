# Section 2: OTP

## Motivation

In this section, we focus on the usage of **OTPs** (One Time Passwords) to protect the access to confidential applications. To motivate this focus, let us look at the following use case. We want to sign a container image with the help of private/public key pair. To protect the private key pair, we want to generate the key pair inside of an enclave and keep the private key always inside of an enclaves.

## Solution

This folder contains the solutions for assignment 1 and 2.

## Assignment 1: Confidential QR code generator program

We use Time based One Time Passwords (TOTP), this means that the one uses a secret and the current time to generate One Time Passwords (OTP). The details of TOTP are described in RFC6238.

TOTP passwords could be stolen as normal passwords. They are, however, only valid for 30 seconds. We will learn that SCONE CAS accepts an OTP only once. This means that after a user has used an OTP neither the user nor an adversary can use this OTP anymore. A user would therefore get an error message that this OTP was already used in case the adversary is faster than the user of sending the OTP.

An adversary could still trick an authorized user to reveal the current OTP using phishing. We therefore recommend that one does not use a single secret (a.k.a. *account*) to authenticate a user. Instead, we recommend that a user allocates an account for each task or role of the user.  Each account has a unique secret and hence, produces a unique sequence of OTPs.

### Problem description

We need to 
The objective of this confidential program `otpqr` that gets the secret...
is to generate either

- a SVG to display the QR code, or
- an URL to visualize the QR code inside of a browser.

### Output

A user should execute this on a computer that is at least temporarily trusted. Still, our objective is that neither the URL nor the SVG is visible in the terminal log. Instead it should be written to an output file.
To ensure that the computer is temporarily trusted. For example, the user might use a dedicated computer,  checks that no other users are logged in, no other programs are running, the user just performed a secure and measured boot, etc.

After scanning QR code with her/his authenticator, the user deletes the file using using some secure file delete function like `shred`:

```bash
shred -n 3 -z -u qrcode.svg
```

### Rust Support

Create a confidential program - actually, a command line utility - that can be execute in an enclave. Use your favorite programming language to do so. In our reference solution, we write this program in Rust. SCONE provides a [cross-compiler for Rust](https://sconedocs.github.io/Rust/). You can build your program with `cargo`.

Rust supports a crate `google_authenticator` which permits to generate SVGs or URLs that can be used to .

### Single Execution Only

We want to generate the QR code only once. After the first execution, one cannot just execute the program `otpqr` a second time to regenerate the output file. To do so, `otpqr` first creates a file with a given path, say, `single_run/once`. If the file already exists, `otpqr` exists with an error since it has already generated the QR code.

If the file `single_run/once` has not yet existed, `otpqr` generates the QR code and stores it in the output file. An adversary with root access, could delete this file. However, the SCONE file shield would detect that file `single_run/once` was removed. This means that `otpqr` would abort typically during attestation. In some more sophisticated attacks, it might only detect when it tries to create file `single_run/once`.

To enforce this **consistency** of the files, we need to enable the SCONE file shield for file `single_run/once`.

## Assignment 2: Build a container image  

Build a container image from the program that you created in task 1. Use a [multistage build](https://sconedocs.github.io/multistagebuild/) to generate a minimal image:

- in the first stage, you use the SCONE Rust cross-compiler image to build you application with `cargo build --release`. If you use a different programming language, you might need to use a different container image.

- in the second stage, you use the binary that was created in the first stage and copy to small container image. In the case of Rust, we create a [statically linked binary](https://sconedocs.github.io/SCONE_toolchain/#statically-linked-binaries) which does not have any external dependencies.

Provide a simple script, say, `build_image.sh` that builds the container image with the help of a Dockerfile.
