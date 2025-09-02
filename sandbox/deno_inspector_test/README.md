### Debug with VSCode

```json
    {
      "name": "Nino",
      "type": "node",
      "request": "launch",
      "cwd": "${workspaceFolder}",
      "runtimeExecutable": "cargo",
      "runtimeArgs": [
        "run"
      ],
      "attachSimplePort": 9229
    }
```