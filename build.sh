#!/bin/bash

if [ "$1" == "front" ] || [ "$1" == "full" ]; then
    cd .
    cd front
    yarn install
    yarn run build:notsc
    cd ..
fi

if [ "$1" == "full" ]; then
    cargo build -r
elif [ "$1" == "base" ];
then
    cd chimes-store-server
    cargo b -r --no-default-features --features "plugin_rlib mqtt mcp synctask rag ragent"
    cd ..
fi

if [ ! -f target/release/store-server ]; then
    cd chimes-store-server
    cargo b -r --no-default-features --features "plugin_rlib mqtt mcp synctask rag ragent"
    cd ..
fi

rm -rf  target/dist
mkdir -p target/dist/assets
cp -r chimes-store-server/assets/* target/dist/assets/
rm -rf target/dist/assets/models/*
cp -r front/dist/* target/dist/assets/management/

if find target/release/*.so* -type f -print -quit | grep -q .; then
    cp target/release/*.so*  target/dist/
fi

cp target/release/chimes-starter target/dist/
cp target/release/store-server target/dist/

opname=`uname`
archname=`arch`
filename=`uname -v | awk '{gsub(/#[0-9]+~/, ""); print $1}'`
fullname="GrowthStore-$opname-$archname-$filename.tar.gz"
if [ -f ./target/$fullname ]; then
    rm -f ./target/$fullname
fi

cd target/dist/

if find *.so* -type f -print -quit | grep -q .; then
    tar -czvf ../$fullname assets/ *.so* chimes-starter  store-server 
else
    tar -czvf ../$fullname assets/  chimes-starter  store-server 
fi
