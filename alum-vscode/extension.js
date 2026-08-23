const fs = require("fs");
const path = require("path");
const vscode = require("vscode");
const {
    LanguageClient,
    TransportKind,
} = require("vscode-languageclient/node");

let client = null;

function activate(context) {
    // 无论配置是否有效都先注册命令，避免命令面板出现悬空命令
    context.subscriptions.push(
        vscode.commands.registerCommand("alum.restartLsp", async () => {
            if (client) {
                await client.stop();
                client = null;
            }
            activate(context);
        })
    );

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

    client.start();
}

function deactivate() {
    if (client) {
        return client.stop();
    }
}

module.exports = { activate, deactivate };
