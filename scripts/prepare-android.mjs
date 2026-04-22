import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const androidAppDir = join('src-tauri', 'gen', 'android', 'app');
const mainActivityPath = join(
  androidAppDir,
  'src',
  'main',
  'java',
  'app',
  'streamkeep',
  'mobile',
  'MainActivity.kt'
);
const appBuildGradlePath = join(androidAppDir, 'build.gradle.kts');

if (!existsSync(androidAppDir)) {
  console.warn('Android project not generated yet; skipping Streamkeep Android preparation.');
  process.exit(0);
}

patchMainActivity();
patchAppBuildGradle();

function patchMainActivity() {
  if (!existsSync(mainActivityPath)) {
    throw new Error(`missing ${mainActivityPath}`);
  }

  let source = readFileSync(mainActivityPath, 'utf8');
  source = ensureAfter(source, 'package app.streamkeep.mobile\n\n', 'import android.content.Context\n');
  source = ensureBefore(
    source,
    '    enableEdgeToEdge()\n',
    '    initializeRustlsPlatformVerifier(applicationContext)\n'
  );
  source = ensureBeforeLast(
    source,
    '}\n',
    '\n  private external fun initializeRustlsPlatformVerifier(context: Context)\n'
  );

  writeFileSync(mainActivityPath, source);
}

function patchAppBuildGradle() {
  if (!existsSync(appBuildGradlePath)) {
    throw new Error(`missing ${appBuildGradlePath}`);
  }

  let source = readFileSync(appBuildGradlePath, 'utf8');
  source = ensureBefore(source, 'import java.util.Properties\n', 'import groovy.json.JsonSlurper\n');
  source = ensureBefore(source, '\nandroid {\n', `${rustlsRepositoryBlock()}\n`);
  source = ensureAfter(
    source,
    'dependencies {\n',
    '    implementation("rustls:rustls-platform-verifier:0.1.1")\n'
  );

  writeFileSync(appBuildGradlePath, source);
}

function rustlsRepositoryBlock() {
  return `\nrepositories {
    maven {
        url = uri(rustlsPlatformVerifierMavenDir())
    }
}

fun rustlsPlatformVerifierMavenDir(): File {
    val dependencyText = providers.exec {
        workingDir = file("../../..")
        commandLine(
            "cargo",
            "metadata",
            "--format-version",
            "1",
            "--filter-platform",
            "aarch64-linux-android",
            "--manifest-path",
            "Cargo.toml"
        )
    }.standardOutput.asText.get()

    @Suppress("UNCHECKED_CAST")
    val packages = (JsonSlurper().parseText(dependencyText) as Map<String, Any>)["packages"] as List<Map<String, Any>>
    val manifestPath = packages
        .first { it["name"] == "rustls-platform-verifier-android" }["manifest_path"]
        .toString()

    return File(File(manifestPath).parentFile, "maven")
}
`;
}

function ensureAfter(source, marker, addition) {
  if (source.includes(addition.trim())) {
    return source;
  }

  const index = source.indexOf(marker);
  if (index === -1) {
    throw new Error(`could not find marker: ${marker.trim()}`);
  }

  return `${source.slice(0, index + marker.length)}${addition}${source.slice(index + marker.length)}`;
}

function ensureBefore(source, marker, addition) {
  if (source.includes(addition.trim())) {
    return source;
  }

  const index = source.indexOf(marker);
  if (index === -1) {
    throw new Error(`could not find marker: ${marker.trim()}`);
  }

  return `${source.slice(0, index)}${addition}${source.slice(index)}`;
}

function ensureBeforeLast(source, marker, addition) {
  if (source.includes(addition.trim())) {
    return source;
  }

  const index = source.lastIndexOf(marker);
  if (index === -1) {
    throw new Error(`could not find marker: ${marker.trim()}`);
  }

  return `${source.slice(0, index)}${addition}${source.slice(index)}`;
}
