cd .
cd front
call yarn install
call yarn run build:pro
cd ..
cargo build -r
cd target/
rmdir /Q /S dist
cd ..
mkdir target\dist
mkdir target\dist\assets
xcopy /Y /E chimes-store-server\assets\** target\dist\assets\
xcopy /Y /E front\dist\** target\dist\assets\management\
copy target\release\*.exe target\dist\
del target\GrowthStore-Win64.zip
powershell -Command "Compress-Archive -Update -Path target\dist\** -DestinationPath target\GrowthStore-Win64.zip"

