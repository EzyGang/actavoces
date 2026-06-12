$ErrorActionPreference = 'Stop'

$manifestPath = 'src-tauri/Cargo.toml'

Write-Host 'Listing Rust library tests'
$listOutput = & cargo test --manifest-path $manifestPath --lib -- --list
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

$testNames = @()
foreach ($line in $listOutput) {
  if ($line -match '^(?<name>.+): test$') {
    $testNames += $Matches.name
  }
}

if ($testNames.Count -eq 0) {
  throw 'No Rust library tests were discovered.'
}

Write-Host "Running $($testNames.Count) Rust library tests in isolated Windows processes"
foreach ($testName in $testNames) {
  Write-Host "::group::$testName"
  & cargo test --manifest-path $manifestPath --lib $testName -- --exact --nocapture
  $testExitCode = $LASTEXITCODE
  Write-Host '::endgroup::'

  if ($testExitCode -ne 0) {
    exit $testExitCode
  }
}

Write-Host 'Running Rust binary tests'
& cargo test --manifest-path $manifestPath --bin actavoces -- --test-threads=1 --nocapture
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}

Write-Host 'Running Rust documentation tests'
& cargo test --manifest-path $manifestPath --doc
if ($LASTEXITCODE -ne 0) {
  exit $LASTEXITCODE
}
