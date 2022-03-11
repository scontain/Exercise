

# Build a native image only
docker build . -t cosign:native -f Dockerfile_native

# Build everything in a single step - leaves build environment which is not optimal
docker build . -t cosign:scone -f Dockerfile

# Build a new image from the native image and throws away the build environment
docker build . -t cosign1:scone -f Dockerfile1
