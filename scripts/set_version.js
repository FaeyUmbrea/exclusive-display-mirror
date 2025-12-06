const fs = require('fs');
const path = require('path');

const tag = process.env.TAG;
if (!tag) {
  console.error('Error: TAG environment variable is not set.');
  process.exit(1);
}

const version = tag.startsWith('v') ? tag.substring(1) : tag;
console.log(`Target version: ${version}`);

// Update package.json
const packageJsonPath = path.resolve(__dirname, '../package.json');
try {
  if (fs.existsSync(packageJsonPath)) {
    const packageJson = JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'));
    const oldVersion = packageJson.version;
    packageJson.version = version;
    fs.writeFileSync(packageJsonPath, JSON.stringify(packageJson, null, 2) + '\n');
    console.log(`Updated package.json: ${oldVersion} -> ${version}`);
  } else {
    console.warn(`Warning: ${packageJsonPath} not found.`);
  }
} catch (error) {
  console.error(`Error updating package.json: ${error.message}`);
  process.exit(1);
}

// Update src-tauri/Cargo.toml
const cargoTomlPath = path.resolve(__dirname, '../src-tauri/Cargo.toml');
try {
  if (fs.existsSync(cargoTomlPath)) {
    let cargoToml = fs.readFileSync(cargoTomlPath, 'utf8');
    const packageHeaderIndex = cargoToml.indexOf('[package]');
    
    if (packageHeaderIndex === -1) {
      console.error('Error: Could not find [package] section in Cargo.toml');
      process.exit(1);
    }

    const prePackage = cargoToml.slice(0, packageHeaderIndex);
    const postPackage = cargoToml.slice(packageHeaderIndex);
    
    // Regex to match version = "..."
    // We use a non-greedy match for the content inside quotes
    const versionRegex = /version\s*=\s*".*?"/;
    
    if (!versionRegex.test(postPackage)) {
         console.error('Error: Could not find version key in [package] section of Cargo.toml');
         process.exit(1);
    }

    const newPostPackage = postPackage.replace(versionRegex, `version = "${version}"`);
    
    fs.writeFileSync(cargoTomlPath, prePackage + newPostPackage);
    console.log(`Updated src-tauri/Cargo.toml version to ${version}`);
  } else {
    console.warn(`Warning: ${cargoTomlPath} not found.`);
  }
} catch (error) {
  console.error(`Error updating Cargo.toml: ${error.message}`);
  process.exit(1);
}
