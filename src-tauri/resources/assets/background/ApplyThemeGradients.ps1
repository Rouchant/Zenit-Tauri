# ApplyThemeGradients.ps1
# Script de PowerShell para procesar los videos de fondo (background-asus.mp4 y background-generic.mp4)
# emulando exactamente el gradiente radial translúcido de CSS usando FFmpeg.

$ErrorActionPreference = "Stop"

# Encontrar FFmpeg
$ffmpeg = "ffmpeg"
if (!(Get-Command $ffmpeg -ErrorAction SilentlyContinue)) {
    $wingetFfmpeg = "$env:USERPROFILE\AppData\Local\Microsoft\WinGet\Links\ffmpeg.exe"
    if (Test-Path $wingetFfmpeg) {
        $ffmpeg = $wingetFfmpeg
    } else {
        Write-Error "No se encontró FFmpeg en el PATH ni en la ruta predeterminada de WinGet. Por favor instálalo o añádelo al PATH."
    }
}

# Crear carpeta de salida
$outputDir = ".\zenit_output"
if (!(Test-Path $outputDir)) {
    New-Item -ItemType Directory -Path $outputDir | Out-Null
}

# Obtener videos de fondo en la carpeta actual
$videos = Get-ChildItem -File | Where-Object { $_.Name -match '^background-(asus|generic)\.(mp4|mov|avi|mkv)$' }

if ($videos.Count -eq 0) {
    Write-Host "⚠️ No se encontraron videos 'background-asus' o 'background-generic' en el directorio actual." -ForegroundColor Yellow
    Exit
}

# Fórmulas de filtros FFmpeg para cada tema optimizadas:
# - Se incrementa el divisor del radio a W*0.95 para empujar el degradado oscuro hacia las esquinas exteriores.
# - El video de fondo original es oscuro, por lo que incrementamos la opacidad del color del tema para cubrirlo mejor.
# - Opacidad configurada: 0.60 en el centro para dar viveza al color y 0.85 en las esquinas para mantener el marco controlado.
$themes = @{
    "default" = @{
        "a" = "(0.6 + 0.25 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "r" = "(5 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "g" = "(242 - 207 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "b" = "(170 - 145 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
    };
    "falabella" = @{
        "a" = "(0.6 + 0.25 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "r" = "(197 - 172 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "g" = "(227 - 197 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "b" = "(5 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
    };
    "paris" = @{
        "a" = "(0.6 + 0.25 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "r" = "(5 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "g" = "(209 - 184 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "b" = "(255 - 220 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
    };
    "ripley" = @{
        "a" = "(0.6 + 0.25 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "r" = "(175 - 150 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "g" = "(71 - 61 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
        "b" = "(255 - 215 * min(hypot(X-W*0.5,Y-H*0.2)/(W*0.95),1))"
    }
}

foreach ($video in $videos) {
    $name = $video.BaseName
    $ext = $video.Extension
    
    foreach ($themeName in $themes.Keys) {
        $theme = $themes[$themeName]
        $a = $theme["a"]
        $r = $theme["r"]
        $g = $theme["g"]
        $b = $theme["b"]
        
        $outputFile = Join-Path $outputDir "$($name)_$($themeName)$($ext)"
        Write-Host "🛍️ Procesando gradiente radial real ($themeName) para: $($video.Name)" -ForegroundColor Cyan
        
        # Filtro de conversión a RGB, cálculo radial usando geq, y reconversión a YUV420p para compatibilidad
        $filter = "format=rgb24,geq=" +
                  "r='r(X,Y)*(1-$a) + ($r)*$a':" +
                  "g='g(X,Y)*(1-$a) + ($g)*$a':" +
                  "b='b(X,Y)*(1-$a) + ($b)*$a'," +
                  "format=yuv420p"

        # Ejecutar FFmpeg
        & $ffmpeg -y -i $video.FullName -vf $filter -c:v libx264 -crf 20 -c:a copy $outputFile
    }
}

Write-Host "✅ ¡Todos los entornos de retail exportados manteniendo el video de fondo!" -ForegroundColor Green
