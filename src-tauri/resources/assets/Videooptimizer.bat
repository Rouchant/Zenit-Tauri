@echo off
title Optimizador de Videos para Kiosco (Zenit)
echo ====================================================
echo    OPTIMIZADOR DE VIDEOS PARA KIOSCO (FFMPEG)
echo ====================================================
echo.

:: Definir la ruta exacta de tu FFmpeg de WinGet
set "FFMPEG_PATH=C:\Users\jmema\AppData\Local\Microsoft\WinGet\Links\ffmpeg.exe"

:: Crear la carpeta de salida si no existe
if not exist "videos_kiosco" (
    mkdir "videos_kiosco"
    echo [+] Carpeta 'videos_kiosco' creada con exito.
)

echo [+] Procesando archivos... esto puede tomar unos segundos por video.
echo.

:: Bucle para procesar cada archivo .mp4
for %%f in (*.mp4) do (
    echo [Procesando] "%%f" ...
    
    :: Se usa la variable %FFMPEG_PATH% entre comillas por seguridad
    "%FFMPEG_PATH%" -y -i "%%f" -an -vcodec libx264 -crf 23 -movflags +faststart "videos_kiosco\%%f" >nul 2>&1
    
    echo [ OK ] "%%f" guardado en 'videos_kiosco'.
    echo ----------------------------------------------------
)

echo.
echo ====================================================
echo     ¡PROCESO COMPLETADO CON EXITO!
echo ====================================================
echo Los videos optimizados estan en la carpeta 'videos_kiosco'.
echo.
pause