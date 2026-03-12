"use strict";
var __createBinding = (this && this.__createBinding) || (Object.create ? (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    var desc = Object.getOwnPropertyDescriptor(m, k);
    if (!desc || ("get" in desc ? !m.__esModule : desc.writable || desc.configurable)) {
      desc = { enumerable: true, get: function() { return m[k]; } };
    }
    Object.defineProperty(o, k2, desc);
}) : (function(o, m, k, k2) {
    if (k2 === undefined) k2 = k;
    o[k2] = m[k];
}));
var __setModuleDefault = (this && this.__setModuleDefault) || (Object.create ? (function(o, v) {
    Object.defineProperty(o, "default", { enumerable: true, value: v });
}) : function(o, v) {
    o["default"] = v;
});
var __importStar = (this && this.__importStar) || (function () {
    var ownKeys = function(o) {
        ownKeys = Object.getOwnPropertyNames || function (o) {
            var ar = [];
            for (var k in o) if (Object.prototype.hasOwnProperty.call(o, k)) ar[ar.length] = k;
            return ar;
        };
        return ownKeys(o);
    };
    return function (mod) {
        if (mod && mod.__esModule) return mod;
        var result = {};
        if (mod != null) for (var k = ownKeys(mod), i = 0; i < k.length; i++) if (k[i] !== "default") __createBinding(result, mod, k[i]);
        __setModuleDefault(result, mod);
        return result;
    };
})();
Object.defineProperty(exports, "__esModule", { value: true });
exports.HydraStatusBar = void 0;
const vscode = __importStar(require("vscode"));
class HydraStatusBar {
    constructor() {
        this.state = 'idle';
        this.item = vscode.window.createStatusBarItem(vscode.StatusBarAlignment.Left, 100);
        this.item.command = 'hydra.toggleSidebar';
        this.update('idle');
        this.item.show();
    }
    update(state, detail) {
        this.state = state;
        const icon = this.getIcon(state);
        const text = this.getText(state, detail);
        this.item.text = `${icon} ${text}`;
        this.item.tooltip = this.getTooltip(state);
        this.item.color = this.getColor(state);
    }
    getState() {
        return this.state;
    }
    getIcon(state) {
        switch (state) {
            case 'idle': return '$(circle-filled)';
            case 'listening': return '$(pulse)';
            case 'working': return '$(sync~spin)';
            case 'needsAttention': return '$(bell)';
            case 'approvalNeeded': return '$(shield)';
            case 'success': return '$(check)';
            case 'error': return '$(error)';
            case 'offline': return '$(circle-slash)';
        }
    }
    getText(state, detail) {
        const base = 'Hydra';
        if (detail) {
            return `${base}: ${detail}`;
        }
        switch (state) {
            case 'idle': return base;
            case 'listening': return `${base}: Listening`;
            case 'working': return `${base}: Working`;
            case 'needsAttention': return `${base}: Attention`;
            case 'approvalNeeded': return `${base}: Approval`;
            case 'success': return `${base}: Done`;
            case 'error': return `${base}: Error`;
            case 'offline': return `${base}: Offline`;
        }
    }
    getTooltip(state) {
        switch (state) {
            case 'idle': return 'Hydra is connected and idle';
            case 'listening': return 'Hydra is listening for input';
            case 'working': return 'Hydra is executing a run';
            case 'needsAttention': return 'Hydra needs your attention';
            case 'approvalNeeded': return 'Hydra has a pending approval';
            case 'success': return 'Hydra completed successfully';
            case 'error': return 'Hydra encountered an error';
            case 'offline': return 'Hydra server is not reachable';
        }
    }
    getColor(state) {
        switch (state) {
            case 'idle': return undefined;
            case 'listening': return '#4A9EFF';
            case 'working': return '#4A9EFF';
            case 'needsAttention': return '#FFCC00';
            case 'approvalNeeded': return '#FF8C00';
            case 'success': return '#4EC94E';
            case 'error': return '#F44747';
            case 'offline': return '#888888';
        }
    }
    dispose() {
        this.item.dispose();
    }
}
exports.HydraStatusBar = HydraStatusBar;
//# sourceMappingURL=statusBar.js.map