#!/bin/bash
npm install -g yarn
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
cp target/release/chimes-starter target/dist/
cp target/release/store-server target/dist/
rm -f target/GrowthStore-Linux.zip
cd target/dist/
tar -czvf ../GrowthStore-Linux.tar.gz assets/  chimes-starter  store-server 

