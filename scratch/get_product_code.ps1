$msiPath = "src-tauri\target\release\bundle\msi\Zenit_1.2.7_x64_es-ES.msi"
$msi = New-Object -ComObject WindowsInstaller.Installer
$database = $msi.OpenDatabase($msiPath, 0)
$view = $database.OpenView("SELECT Value FROM Property WHERE Property = 'ProductCode'")
$view.Execute()
$record = $view.Fetch()
if ($record) {
    $record.StringData(1)
}
$view.Close()
