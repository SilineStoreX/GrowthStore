@echo off
set current_driver=%~d0
set current_path=%~p0

%current_driver%
cd %current_path%

cd .
cd front
call yarn install
call yarn run build:notsc
cd ..
cargo b -r --no-default-features --features "plugin_rlib mqtt mcp synctask"
cd target/
rmdir /Q /S dist
cd ..
mkdir target\dist
mkdir target\dist\assets
xcopy /Y /E chimes-store-server\assets\** target\dist\assets\
rmdir /Q /S target\dist\assets\models\
mkdir target\dist\assets\models\
xcopy /Y /E front\dist\** target\dist\assets\management\
copy target\release\*.dll target\dist\
copy target\release\*.exe target\dist\
del target\GrowthStore-Win64.zip
powershell -Command "Compress-Archive -Update -Path target\dist\** -DestinationPath target\GrowthStore-Win64.zip"
pause

