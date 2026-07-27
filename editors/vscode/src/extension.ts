import * as path from 'path';
import * as fs from 'fs';
import * as os from 'os';
import { spawnSync } from 'child_process';
import { workspace, window, commands, ExtensionContext } from 'vscode';

import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  Executable
} from 'vscode-languageclient/node';

let client: LanguageClient;

function findFerriteExecutable(): string | undefined {
  const config = workspace.getConfiguration('ferrite');
  const userConfigured = config.get<string>('executablePath');
  
  // 1. If user explicitly configured something other than default 'ferrite', trust it
  if (userConfigured && userConfigured !== 'ferrite') {
    return userConfigured;
  }

  // 2. Check system PATH
  const isWindows = process.platform === 'win32';
  const binName = isWindows ? 'ferrite.exe' : 'ferrite';
  
  try {
    const checkCmd = isWindows ? 'where' : 'which';
    const result = spawnSync(checkCmd, [binName]);
    if (result.status === 0) {
      return binName; // Found in PATH
    }
  } catch (e) {
    // ignore
  }

  // 3. Check local workspace (for developers building ferrite locally)
  if (workspace.workspaceFolders) {
    for (const folder of workspace.workspaceFolders) {
      const releasePath = path.join(folder.uri.fsPath, 'target', 'release', binName);
      if (fs.existsSync(releasePath)) return releasePath;
      
      const debugPath = path.join(folder.uri.fsPath, 'target', 'debug', binName);
      if (fs.existsSync(debugPath)) return debugPath;
    }
  }

  return undefined;
}

export function activate(context: ExtensionContext) {
  let command = findFerriteExecutable();
  
  if (!command) {
    window.showErrorMessage(
      "Ferrite compiler not found. Please install it or configure 'ferrite.executablePath'.",
      "Open Settings"
    ).then(selection => {
      if (selection === "Open Settings") {
        commands.executeCommand('workbench.action.openSettings', 'ferrite.executablePath');
      }
    });
    return;
  }

  // Windows File Lock Workaround: 
  // Copy ferrite.exe to a temp folder and run the copy so the original can be overwritten/deleted
  if (process.platform === 'win32') {
    try {
      const tmpDir = os.tmpdir();
      // Attempt to clean up old dead copies first
      for (const f of fs.readdirSync(tmpDir)) {
        if (f.startsWith('ferrite-lsp-') && f.endsWith('.exe')) {
          try { fs.unlinkSync(path.join(tmpDir, f)); } catch (_) { /* Ignore if still running */ }
        }
      }
      
      const tmpPath = path.join(tmpDir, `ferrite-lsp-${Date.now()}.exe`);
      fs.copyFileSync(command, tmpPath);
      command = tmpPath;
    } catch (e) {
      console.warn('Failed to create temp copy of ferrite.exe, falling back to original which may lock.', e);
    }
  }

  const run: Executable = {
    command,
    args: ['lsp'],
    options: { env: process.env }
  };
  
  const serverOptions: ServerOptions = {
    run,
    debug: run
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: 'file', language: 'ferrite' }],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher('**/.clientrc')
    }
  };

  client = new LanguageClient(
    'ferriteLanguageServer',
    'Ferrite Language Server',
    serverOptions,
    clientOptions
  );

  client.start();
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}
