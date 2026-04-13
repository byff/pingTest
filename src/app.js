/**
 * PingTest - App Module
 * Application logic and Tauri invoke wrappers
 */

/**
 * @typedef {Object} TargetInfo
 * @property {number} index
 * @property {string} hostname
 * @property {string} ip
 * @property {number} success_count
 * @property {number} fail_count
 * @property {number} total_sent
 * @property {number|null} last_rtt_ms
 * @property {number} max_rtt_ms
 * @property {number} min_rtt_ms
 * @property {number} avg_rtt_ms
 * @property {number} fail_rate
 * @property {boolean} is_alive
 */

/**
 * @typedef {Object} PingConfig
 * @property {number} timeout_ms
 * @property {number} packet_size
 * @property {number} interval_ms
 * @property {number} max_concurrent
 */

/**
 * @typedef {Object} StatsSummary
 * @property {number} total_targets
 * @property {number} alive_count
 * @property {number} dead_count
 * @property {number} total_success
 * @property {number} total_fail
 * @property {boolean} running
 */

class PingTestApp {
    constructor() {
        this.state = {
            targets: [],
            isRunning: false,
            statsInterval: null,
            config: null
        };
    }

    // =========================================================================
    // Tauri Invoke Wrappers
    // =========================================================================

    async getConfig() {
        return await invoke('get_config');
    }

    async saveConfig(config) {
        return await invoke('save_config', { config });
    }

    async updatePingConfig(pingConfig) {
        return await invoke('update_ping_config', { pingConfig });
    }

    async parseInputTargets(input, stripFirstLast) {
        return await invoke('parse_input_targets', { input, stripFirstLast });
    }

    async setTargets(targetsInput) {
        return await invoke('set_targets', { targetsInput });
    }

    async startPing() {
        return await invoke('start_ping');
    }

    async stopPing() {
        return await invoke('stop_ping');
    }

    async getPingStats() {
        return await invoke('get_ping_stats');
    }

    async isPingRunning() {
        return await invoke('is_ping_running');
    }

    async getStatsSummary() {
        return await invoke('get_stats_summary');
    }

    async readExcelFile(path) {
        return await invoke('read_excel_file', { path });
    }

    async exportToExcel(path) {
        return await invoke('export_to_excel', { path });
    }

    async cleanIpInput(input) {
        return await invoke('clean_ip_input', { input });
    }

    async countTargets(input) {
        return await invoke('count_targets', { input });
    }

    // =========================================================================
    // Utility Methods
    // =========================================================================

    formatNumber(num) {
        if (num === null || num === undefined || num === 0) return '-';
        return num.toLocaleString('en-US', { maximumFractionDigits: 2 });
    }

    formatRtt(ms) {
        if (ms === null || ms === undefined || ms === 0) return '-';
        return ms.toFixed(2);
    }

    formatPercent(val) {
        if (val === null || val === undefined) return '0%';
        return val.toFixed(1) + '%';
    }

    debounce(fn, delay) {
        let timeoutId;
        return function(...args) {
            clearTimeout(timeoutId);
            timeoutId = setTimeout(() => fn.apply(this, args), delay);
        };
    }

    // =========================================================================
    // State Management
    // =========================================================================

    setTargets(targets) {
        this.state.targets = targets;
    }

    setRunning(running) {
        this.state.isRunning = running;
    }

    setConfig(config) {
        this.state.config = config;
    }

    getTargets() {
        return this.state.targets;
    }

    isRunning() {
        return this.state.isRunning;
    }

    // =========================================================================
    // App Lifecycle
    // =========================================================================

    async initialize() {
        console.log('PingTestApp: Initializing...');
        
        try {
            // Load configuration
            const config = await this.getConfig();
            this.setConfig(config);
            console.log('PingTestApp: Config loaded', config);
            
            // Check if ping is already running
            const running = await this.isPingRunning();
            this.setRunning(running);
            
            if (running) {
                this.startStatsPolling();
            }
            
            return config;
        } catch (err) {
            console.error('PingTestApp: Initialization error', err);
            throw err;
        }
    }

    startStatsPolling(intervalMs = 1000) {
        if (this.state.statsInterval) {
            clearInterval(this.state.statsInterval);
        }
        
        this.state.statsInterval = setInterval(async () => {
            try {
                const stats = await this.getPingStats();
                const summary = await this.getStatsSummary();
                this.onStatsUpdate(stats, summary);
            } catch (err) {
                console.error('PingTestApp: Stats polling error', err);
            }
        }, intervalMs);
    }

    stopStatsPolling() {
        if (this.state.statsInterval) {
            clearInterval(this.state.statsInterval);
            this.state.statsInterval = null;
        }
    }

    // =========================================================================
    // Event Callbacks (Override these in UI layer)
    // =========================================================================

    onStatsUpdate(stats, summary) {
        // Override in UI to handle stats updates
        console.log('Stats update:', { stats, summary });
    }

    onRunningStateChange(running) {
        // Override in UI to handle running state changes
        console.log('Running state:', running);
    }

    onError(error) {
        // Override in UI to handle errors
        console.error('App error:', error);
    }
}

// Export singleton instance
window.PingTestApp = new PingTestApp();
