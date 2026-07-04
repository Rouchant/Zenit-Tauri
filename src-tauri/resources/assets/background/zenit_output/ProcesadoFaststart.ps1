# Optimizador de Videos para Kiosco (Zenit) en PowerShell
Write-Host "====================================================" -ForegroundColor Yellow
Write-Host "    OPTIMIZADOR DE VIDEOS PARA KIOSCO (FFMPEG)" -ForegroundColor Yellow
Write-Host "====================================================" -ForegroundColor Yellow
Write-Host ""

# Definir la ruta exacta de tu FFmpeg de WinGet
$FFMPEG_PATH = "C:\Users\jmema\AppData\Local\Microsoft\WinGet\Links\ffmpeg.exe"
$OUTPUT_FOLDER = ".\videosprocesados"

Write-Host "[+] Buscando archivos .mp4..." -ForegroundColor Cyan
$videos = Get-ChildItem -File | Where-Object { $_.Extension -eq '.mp4' }

if ($videos.Count -eq 0) {
    Write-Host "[-] No se encontraron archivos .mp4 en este directorio." -ForegroundColor Red
} else {
    Write-Host "[+] Procesando $($videos.Count) archivos... esto puede tomar unos segundos por video.`n" -ForegroundColor Cyan

    # Crear la carpeta única de salida si no existe
    if (-not (Test-Path -Path $OUTPUT_FOLDER)) {
        New-Item -ItemType Directory -Path $OUTPUT_FOLDER | Out-Null
        Write-Host "[+] Carpeta '$OUTPUT_FOLDER' creada con éxito.`n" -ForegroundColor Green
    }

    foreach ($video in $videos) {
        $fileName = $video.Name # Mantiene el nombre completo original (ej: background-asus.mp4)

        Write-Host "[Procesando] `"$fileName`" ..." -ForegroundColor White

        # Ejecutar FFmpeg enviando el resultado directo a la carpeta centralizada
        & $FFMPEG_PATH -y -i "$fileName" -an -vcodec libx264 -crf 23 -movflags +faststart "$OUTPUT_FOLDER\$fileName" *>$null

        Write-Host "[ OK ] `"$fileName`" guardado en '$OUTPUT_FOLDER'." -ForegroundColor Green
        Write-Host "----------------------------------------------------" -ForegroundColor DarkGray
    }

    Write-Host "`n====================================================" -ForegroundColor Green
    Write-Host "     ¡PROCESO COMPLETADO CON EXITO!" -ForegroundColor Green
    Write-Host "====================================================" -ForegroundColor Green
    Write-Host "Todos los videos optimizados están en la carpeta '$OUTPUT_FOLDER'.`n"
}

Read-Host "Presione Enter para salir..."