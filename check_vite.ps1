try {
  $r = Invoke-WebRequest -Uri 'http://localhost:1420/' -UseBasicParsing -TimeoutSec 5
  Write-Output ('Status: ' + $r.StatusCode)
  $content = $r.Content
  if ($content.Length -gt 2000) { $content = $content.Substring(0, 2000) }
  Write-Output $content
} catch {
  Write-Output ('Error: ' + $_.Exception.Message)
}
