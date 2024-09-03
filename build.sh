#!/bin/bash

cd .
cd front
yarn install
yarn run build:pro
cd ..
cargo build -r
rm -rf  target/dist
mkdir -p target/dist/assets
cp -r chimes-store-server/assets/* target/dist/assets/
cp -r front/dist/* target/dist/assets/management/
cp -y target/release/chimes-start target/dist/
cp -y target/release/store-server target/dist/
rm -f target/GrowthStore-Linux.zip
tar -czvf target/GrowthStore-Linux.tar.gz  target/dist/*

