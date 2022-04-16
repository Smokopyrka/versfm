# VersFM

## Overview

**VersFM** is a versitile file manager written in Rust, that allows you to move, copy and delete files within your local filesystem, between your local filesystem and cloud object stores such as AWS S3, or even between object stores located in two different clouds!

![Imgur Image](https://imgur.com/vPps0GZ.gif)

## Notes

- The project **currently supports only file transfer between AWS S3 and the local filesystem**, but due to the use of trait objects, adding the capability to transfer to/from other object stores etc. should only require writing an additional list component and provider. 
- This is my first ever TUI project and also my first ever Rust project, so it's probably far from perfect :).

## Ideas for future development

- Adding detailed view for files, that will display information such as file permissions, ownership etc.
- Implementing other providers for services such as Google Cloud Storage, OneDrive etc.
- Adding command line arguments allowing for customizability and selection of providers (eg. selecting which two providers the user wants to transfer files between etc.)
- Adding support for AWS profiles.
- Adding support for different AWS authentication methods.
