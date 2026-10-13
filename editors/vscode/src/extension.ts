// Minimal Cadetta language client: starts `cadetta lsp` (config-cli.md 3.8).
import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let output: vscode.LogOutputChannel | undefined;

/** Build the client for the configured executable and start it. */
async function startClient(): Promise<void> {
  const config = vscode.workspace.getConfiguration("cadetta");
  if (!config.get<boolean>("lsp.enabled", true)) {
    output?.info("cadetta lsp disabled by cadetta.lsp.enabled");
    return;
  }
  const command = config.get<string>("path", "cadetta");
  const serverOptions: ServerOptions = { command, args: ["lsp"] };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [{ scheme: "file", language: "cadetta" }],
    outputChannel: output,
  };
  client = new LanguageClient("cadetta", "Cadetta", serverOptions, clientOptions);
  output?.info(`starting: ${command} lsp`);
  try {
    await client.start();
  } catch (err) {
    output?.error(`failed to start ${command} lsp: ${String(err)}`);
    void vscode.window.showErrorMessage(
      `Cadetta: could not start "${command} lsp" (see the Cadetta output). Set cadetta.path.`,
    );
    client = undefined;
  }
}

/** Stop the running client, if any. */
async function stopClient(): Promise<void> {
  const running = client;
  client = undefined;
  if (running) {
    await running.stop();
  }
}

/** Activate on the first .cdt document. */
export async function activate(context: vscode.ExtensionContext): Promise<void> {
  output = vscode.window.createOutputChannel("Cadetta", { log: true });
  context.subscriptions.push(output);
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration(async (event) => {
      if (event.affectsConfiguration("cadetta")) {
        output?.info("configuration changed, restarting");
        await stopClient();
        await startClient();
      }
    }),
  );
  await startClient();
}

/** Stop the language client on shutdown. */
export async function deactivate(): Promise<void> {
  await stopClient();
}
