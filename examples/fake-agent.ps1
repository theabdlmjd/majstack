$prompt = [Console]::In.ReadToEnd()
if ($prompt -match '# PLANNING') {
    Write-Output '<tasks>[{"id":"1","title":"create demo files","depends":[]}]</tasks>'
    Write-Output 'MAJSTACK_DONE: planned'
    exit 0
}
if ($prompt -match 'review the task diff') {
    Write-Output 'VERDICT: APPROVE'
    Write-Output 'MAJSTACK_DONE: reviewed'
    exit 0
}
Set-Content -Path 'a.txt' -Value 'alpha'
Set-Content -Path 'b.txt' -Value 'beta'
Write-Output 'MAJSTACK_LEARN: PATTERN: demo artifacts are plain text files'
Write-Output 'MAJSTACK_DONE: created demo files'
