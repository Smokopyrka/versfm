# VersFM

## Overview

**VersFM** is a versatile file manager written in Rust, that allows you to move, copy and delete files within your local filesystem, between your local filesystem and cloud object stores such as AWS S3, or even between object stores located in two different clouds!

![Imgur Image](https://i.imgur.com/MdJoSUq.gif)

## Usage

Running the tool without any flags will launch it into regular dual-pane file manager mode. If you want to connect to a specific object store, you will have to run versfm with some flags. First, you have to select the provider you want to use (**currently only AWS S3 and local filesystem provides are implemented**). To do that, use either the `-l <provider>` or `-r <provider>` flags to indicate the provider you want to use and whether it should be displayed in the right or left pane. Additionally you'll have to provide provider-specific options (such as `aws-region` and `s3-bucket-name` in the case of the S3 provider).

Navigating inside the application is done in the following way:
- `LEFT_ARROW` or `H` - Move to the left pane
- `RIGHT_ARROW` or `L` - Move to the right pane
- `UP_ARROW` or `K` - Move up in the currently selected list
- `DOWN_ARROW` or `J` - Move down in the currently selected list
- `SPACE` - go into the directory currently under cursor
- `BACKSPACE` - move out of the current directory
- `M` - mark file currently under cursor for move
- `D` - mark file currently under cursor for deletion
- `C` - mark file currently under cursor for copying
- `R` - refresh both lists
- `ENTER` - perform operations (move, delete, copy)
- `ESC` - exit the application

## Notes

- The project **currently supports only file transfer to-and-from AWS S3 and the local filesystem**, but due to the use of trait objects, adding the capability to transfer to/from other object stores etc. should only require writing an additional list component and provider. 
- This is my first ever TUI project and also my first ever Rust project, so it's probably far from perfect :).

## Ideas for future development

- Adding detailed view for files, that will display information such as file permissions, ownership etc.
- Implementing other providers for services such as Google Cloud Storage, OneDrive etc.
- Adding support for AWS profiles.
- Adding support for different AWS authentication methods.
