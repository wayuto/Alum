const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const {
    LanguageClient,
    TransportKind,
} = require("vscode-languageclient/node");

let client = null;

function activate(context) {
    const config = vscode.workspace.getConfiguration("alum");
    const serverPath = config.get("lsp.path", "alum-lsp");
    if (path.isAbsolute(serverPath) && !fs.existsSync(serverPath)) {
        vscode.window.showErrorMessage(
            `alum-lsp not found at '${serverPath}'. ` +
            "Install it with 'cargo install --path <alc-repo>' " +
            "or set 'alum.lsp.path' to the correct location."
        );
        return;
    }

    const serverOptions = {
        command: serverPath,
        args: [],
        transport: TransportKind.stdio,
    };

    const clientOptions = {
        documentSelector: [{ scheme: "file", language: "alum" }],
        synchronize: {
            configurationSection: "alum",
        },
        outputChannelName: "Alum Language Server",
    };

    client = new LanguageClient(
        "alum-lsp",
        "Alum Language Server",
        serverOptions,
        clientOptions
    );

    context.subscriptions.push(
        vscode.commands.registerCommand("alum.restartLsp", () => {
            if (client) {
                client.restart();
            }
        })
    );

    client.start();
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
