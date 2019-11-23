FROM rustlang/rust:nightly

RUN apt-get update; \ 
    apt-get install -y --no-install-recommends clang libvips-dev 

WORKDIR /img-storage

COPY . .

RUN cargo install --path .

ENV PORT=3000
ENV RUST_LOG=info
ENV IMG_FOLDER=/mnt/images
CMD ["img-storage"]
