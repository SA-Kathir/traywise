# Measures traywise.exe plus every process it spawned (the WebView2 helpers).
# Usage: powershell -NoProfile -ExecutionPolicy Bypass -File scripts\measure-memory.ps1

$root = Get-CimInstance Win32_Process | Where-Object { $_.Name -eq "traywise.exe" }
if (-not $root) { Write-Output "traywise.exe is not running"; exit 1 }

$all = Get-CimInstance Win32_Process
$family = @($root.ProcessId)
do {
    $before = $family.Count
    $children = $all | Where-Object { $family -contains $_.ParentProcessId -and $family -notcontains $_.ProcessId }
    $family = @($family + ($children | ForEach-Object { $_.ProcessId }))
} while ($family.Count -ne $before)

$procs = $family | ForEach-Object { Get-Process -Id $_ -ErrorAction SilentlyContinue }
$procs | Select-Object Name, Id,
    @{ n = "WorkingSetMB"; e = { [math]::Round($_.WorkingSet64 / 1MB, 1) } },
    @{ n = "PrivateMB"; e = { [math]::Round($_.PrivateMemorySize64 / 1MB, 1) } },
    @{ n = "CPUsec"; e = { [math]::Round($_.CPU, 2) } } |
    Format-Table -AutoSize | Out-String -Width 120

$ws = ($procs | Measure-Object WorkingSet64 -Sum).Sum / 1MB
$pv = ($procs | Measure-Object PrivateMemorySize64 -Sum).Sum / 1MB
$cpu = ($procs | Measure-Object CPU -Sum).Sum
"Total working set: {0:N0} MB | Total private: {1:N0} MB | CPU: {2:N2} s | Processes: {3}" -f $ws, $pv, $cpu, $procs.Count
