/**
 * PingTest - Main JavaScript Module
 * Vanilla JS + Tauri 2.x API
 */

const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;
const { readFile, writeFile } = window.__TAURI__.fs;

// =============================================================================
// State Management
// =============================================================================

const state = {
    targets: [],
    isRunning: false,
    statsInterval: null,
    lastAddresses: []
};

// =============================================================================
// DOM Elements
// =============================================================================

const elements = {
    // Input elements
    targetInput: document.getElementById('targetInput'),
    stripFirstLast: document.getElementById('stripFirstLast'),
    timeoutMs: document.getElementById('timeoutMs'),
    intervalMs: document.getElementById('intervalMs'),
    packetSize: document.getElementById('packetSize'),
    maxConcurrent: document.getElementById('maxConcurrent'),
    targetCount: document.getElementById('targetCount'),
    
    // Buttons
    btnCountTargets: document.getElementById('btnCountTargets'),
    btnClearInput: document.getElementById('btnClearInput'),
    btnLoadExcel: document.getElementById('btnLoadExcel'),
    btnSetTargets: document.getElementById('btnSetTargets'),
    btnStartPing: document.getElementById('btnStartPing'),
    btnStopPing: document.getElementById('btnStopPing'),
    btnExportExcel: document.getElementById('btnExportExcel'),
    btnRefreshStats: document.getElementById('btnRefreshStats'),
    
    // Stats
    statTotal: document.getElementById('statTotal'),
    statAlive: document.getElementById('statAlive'),
    statDead: document.getElementById('statDead'),
    statSuccessRate: document.getElementById('statSuccessRate'),
    statSuccessCount: document.getElementById('statSuccessCount'),
    statFailCount: document.getElementById('statFailCount'),
    
    // Status
    connectionStatus: document.getElementById('connectionStatus'),
    runningStatus: document.getElementById('runningStatus'),
    
    // Table
    resultsBody: document.getElementById('resultsBody'),
    searchFilter: document.getElementById('searchFilter'),
    
    // Overlays
    loadingOverlay: document.getElementById('loadingOverlay'),
    toast: document.getElementById('toast')
};

// =============================================================================
// Utility Functions
// =============================================================================

function formatNumber(num) {
    if (num === null || num === undefined || num === 0) return '-';
    return num.toLocaleString('en-US', { maximumFractionDigits: 2 });
}

function formatRtt(ms) {
    if (ms === null || ms === undefined || ms === 0) return '-';
    return ms.toFixed(2);
}

function formatPercent(val) {
    if (val === null || val === undefined) return '0%';
    return val.toFixed(1) + '%';
}

function debounce(fn, delay) {
    let timeoutId;
    return function(...args) {
        clearTimeout(timeoutId);
        timeoutId = setTimeout(() => fn.apply(this, args), delay);
    };
}

// =============================================================================
// Toast Notifications
// =============================================================================

function showToast(message, type = 'info', duration = 3000) {
    const toast = elements.toast;
    toast.textContent = message;
    toast.className = `toast toast-${type}`;
    toast.classList.remove('hidden');
    
    setTimeout(() => {
        toast.classList.add('hidden');
    }, duration);
}

// =============================================================================
// Loading Overlay
// =============================================================================

function showLoading(text = '处理中...') {
    const overlay = elements.loadingOverlay;
    overlay.querySelector('.loading-text').textContent = text;
    overlay.classList.remove('hidden');
}

function hideLoading() {
    elements.loadingOverlay.classList.add('hidden');
}

// =============================================================================
// Status Updates
// =============================================================================

function updateConnectionStatus(status) {
    const el = elements.connectionStatus;
    el.className = 'status-indicator';
    
    switch (status) {
        case 'connected':
            el.classList.add('status-connected');
            el.textContent = '已连接';
            break;
        case 'disconnected':
            el.classList.add('status-disconnected');
            el.textContent = '未连接';
            break;
        case 'running':
            el.classList.add('status-running');
            el.textContent = '运行中';
            break;
    }
}

function updateRunningStatus(running) {
    state.isRunning = running;
    
    elements.btnStartPing.disabled = running || state.targets.length === 0;
    elements.btnStopPing.disabled = !running;
    
    if (running) {
        elements.runningStatus.textContent = '状态: Ping运行中';
        updateConnectionStatus('running');
    } else {
        elements.runningStatus.textContent = '状态: 空闲';
        updateConnectionStatus(state.targets.length > 0 ? 'connected' : 'disconnected');
    }
}

// =============================================================================
// Stats Display
// =============================================================================

function updateStatsDisplay(stats) {
    elements.statTotal.textContent = stats.total_targets || 0;
    elements.statAlive.textContent = stats.alive_count || 0;
    elements.statDead.textContent = stats.dead_count || 0;
    elements.statSuccessCount.textContent = stats.total_success || 0;
    elements.statFailCount.textContent = stats.total_fail || 0;
    
    const totalSent = (stats.total_success || 0) + (stats.total_fail || 0);
    const successRate = totalSent > 0 ? ((stats.total_success || 0) / totalSent * 100) : 0;
    elements.statSuccessRate.textContent = formatPercent(successRate);
}

function updateStatsSummary() {
    invoke('get_stats_summary')
        .then(stats => updateStatsDisplay(stats))
        .catch(err => console.error('Failed to get stats summary:', err));
}

// =============================================================================
// Table Rendering
// =============================================================================

function renderTargetsTable(targets, filter = '') {
    const tbody = elements.resultsBody;
    
    if (!targets || targets.length === 0) {
        tbody.innerHTML = `
            <tr class="empty-row">
                <td colspan="11" class="empty-message">暂无数据，请先设置目标并开始Ping测试</td>
            </tr>
        `;
        return;
    }
    
    const filterLower = filter.toLowerCase().trim();
    const filtered = filterLower 
        ? targets.filter(t => 
            t.hostname.toLowerCase().includes(filterLower) ||
            t.ip.toLowerCase().includes(filterLower)
        )
        : targets;
    
    if (filtered.length === 0) {
        tbody.innerHTML = `
            <tr class="empty-row">
                <td colspan="11" class="empty-message">没有匹配 "${filter}" 的结果</td>
            </tr>
        `;
        return;
    }
    
    tbody.innerHTML = filtered.map(target => {
        const statusClass = target.total_sent === 0 
            ? 'status-pending-badge' 
            : target.is_alive 
                ? 'status-alive-badge' 
                : 'status-dead-badge';
        
        const statusText = target.total_sent === 0 
            ? '等待' 
            : target.is_alive 
                ? '在线' 
                : '离线';
        
        return `
            <tr data-index="${target.index}">
                <td class="col-index">${target.index + 1}</td>
                <td class="col-hostname" title="${target.hostname}">${target.hostname || '-'}</td>
                <td class="col-ip" title="${target.ip}">${target.ip}</td>
                <td class="col-status"><span class="status-badge ${statusClass}">${statusText}</span></td>
                <td class="col-success">${formatNumber(target.success_count)}</td>
                <td class="col-fail">${formatNumber(target.fail_count)}</td>
                <td class="col-fail-rate">${formatPercent(target.fail_rate)}</td>
                <td class="col-sent">${formatNumber(target.total_sent)}</td>
                <td class="col-last-rtt">${formatRtt(target.last_rtt_ms)}</td>
                <td class="col-avg-rtt">${formatRtt(target.avg_rtt_ms)}</td>
                <td class="col-min-rtt">${formatRtt(target.min_rtt_ms)}</td>
                <td class="col-max-rtt">${formatRtt(target.max_rtt_ms)}</td>
            </tr>
        `;
    }).join('');
}

function refreshTable() {
    invoke('get_ping_stats')
        .then(stats => {
            state.targets = stats;
            renderTargetsTable(stats, elements.searchFilter.value);
        })
        .catch(err => console.error('Failed to get ping stats:', err));
}

// =============================================================================
// Target Management
// =============================================================================

async function parseAndCountTargets() {
    const input = elements.targetInput.value.trim();
    if (!input) {
        elements.targetCount.textContent = '0';
        return;
    }
    
    showLoading('正在计数...');
    
    try {
        const count = await invoke('count_targets', { input });
        elements.targetCount.textContent = count.toLocaleString();
    } catch (err) {
        console.error('Count error:', err);
        elements.targetCount.textContent = '错误';
        showToast('计数失败: ' + err, 'error');
    } finally {
        hideLoading();
    }
}

async function setTargets() {
    const input = elements.targetInput.value.trim();
    if (!input) {
        showToast('请输入目标地址', 'error');
        return;
    }
    
    const stripFirstLast = elements.stripFirstLast.checked;
    showLoading('正在解析目标...');
    
    try {
        // Parse targets
        const parsed = await invoke('parse_input_targets', { input, stripFirstLast });
        
        if (parsed.length === 0) {
            showToast('未解析到有效目标', 'error');
            hideLoading();
            return;
        }
        
        // Save last addresses for remember_addresses feature
        const addresses = parsed.map(t => t.hostname || t.ip);
        state.lastAddresses = addresses;
        
        // Prepare target inputs for set_targets
        const targetInputs = parsed.map(t => ({
            ip: t.ip,
            hostname: t.hostname
        }));
        
        // Set targets in backend
        await invoke('set_targets', { targetsInput: targetInputs });
        
        // Update config with ping settings
        const pingConfig = {
            timeout_ms: parseInt(elements.timeoutMs.value) || 1000,
            packet_size: parseInt(elements.packetSize.value) || 128,
            interval_ms: parseInt(elements.intervalMs.value) || 1000,
            max_concurrent: parseInt(elements.maxConcurrent.value) || 200
        };
        
        await invoke('update_ping_config', { pingConfig });
        
        // Initial stats fetch
        const stats = await invoke('get_ping_stats');
        state.targets = stats;
        renderTargetsTable(stats);
        updateStatsDisplay({
            total_targets: stats.length,
            alive_count: 0,
            dead_count: 0,
            total_success: 0,
            total_fail: 0
        });
        
        // Enable start button
        elements.btnStartPing.disabled = false;
        updateConnectionStatus('connected');
        
        showToast(`已设置 ${stats.length} 个目标`, 'success');
        
    } catch (err) {
        console.error('Set targets error:', err);
        showToast('设置目标失败: ' + err, 'error');
    } finally {
        hideLoading();
    }
}

// =============================================================================
// Ping Control
// =============================================================================

async function startPing() {
    if (state.isRunning) {
        showToast('Ping已经在运行中', 'info');
        return;
    }
    
    try {
        await invoke('start_ping');
        updateRunningStatus(true);
        
        // Start stats polling
        state.statsInterval = setInterval(() => {
            refreshTable();
            updateStatsSummary();
        }, 1000);
        
        showToast('Ping已启动', 'success');
    } catch (err) {
        console.error('Start ping error:', err);
        showToast('启动失败: ' + err, 'error');
    }
}

async function stopPing() {
    if (!state.isRunning) {
        showToast('Ping未在运行', 'info');
        return;
    }
    
    try {
        await invoke('stop_ping');
        updateRunningStatus(false);
        
        // Stop stats polling
        if (state.statsInterval) {
            clearInterval(state.statsInterval);
            state.statsInterval = null;
        }
        
        // Final refresh
        refreshTable();
        updateStatsSummary();
        
        showToast('Ping已停止', 'success');
    } catch (err) {
        console.error('Stop ping error:', err);
        showToast('停止失败: ' + err, 'error');
    }
}

// =============================================================================
// Excel Import/Export
// =============================================================================

async function loadExcelFile() {
    try {
        const filePath = await open({
            multiple: false,
            filters: [{
                name: 'Excel',
                extensions: ['xlsx', 'xls']
            }]
        });
        
        if (!filePath) return;
        
        showLoading('正在导入Excel...');
        
        const excelData = await invoke('read_excel_file', { path: filePath });
        
        if (!excelData || !excelData.rows || excelData.rows.length === 0) {
            showToast('Excel文件为空或格式错误', 'error');
            hideLoading();
            return;
        }
        
        // Convert Excel data to target input format
        // Assumes first column is hostname/IP, second column could be IP
        const targets = excelData.rows
            .map(row => row[0])
            .filter(cell => cell && cell.trim())
            .join('\n');
        
        elements.targetInput.value = targets;
        
        // Count targets
        await parseAndCountTargets();
        
        showToast(`已导入 ${excelData.rows.length} 行数据`, 'success');
    } catch (err) {
        console.error('Load excel error:', err);
        showToast('导入失败: ' + err, 'error');
    } finally {
        hideLoading();
    }
}

async function exportResults() {
    try {
        const filePath = await save({
            filters: [{
                name: 'Excel',
                extensions: ['xlsx']
            }],
            defaultPath: `ping_results_${new Date().toISOString().slice(0, 10)}.xlsx`
        });
        
        if (!filePath) return;
        
        showLoading('正在导出...');
        
        await invoke('export_to_excel', { path: filePath });
        
        showToast('导出成功', 'success');
    } catch (err) {
        console.error('Export error:', err);
        showToast('导出失败: ' + err, 'error');
    } finally {
        hideLoading();
    }
}

// =============================================================================
// Event Handlers
// =============================================================================

function initEventHandlers() {
    // Target input events
    elements.btnCountTargets.addEventListener('click', parseAndCountTargets);
    
    elements.btnClearInput.addEventListener('click', () => {
        elements.targetInput.value = '';
        elements.targetCount.textContent = '0';
    });
    
    // Target input change - auto count with debounce
    elements.targetInput.addEventListener('input', debounce(parseAndCountTargets, 500));
    
    // Excel buttons
    elements.btnLoadExcel.addEventListener('click', loadExcelFile);
    elements.btnExportExcel.addEventListener('click', exportResults);
    
    // Main control buttons
    elements.btnSetTargets.addEventListener('click', setTargets);
    elements.btnStartPing.addEventListener('click', startPing);
    elements.btnStopPing.addEventListener('click', stopPing);
    
    // Refresh button
    elements.btnRefreshStats.addEventListener('click', () => {
        refreshTable();
        updateStatsSummary();
    });
    
    // Search filter
    elements.searchFilter.addEventListener('input', debounce(() => {
        renderTargetsTable(state.targets, elements.searchFilter.value);
    }, 300));
    
    // Ping settings - update config when changed
    const pingSettings = ['timeoutMs', 'intervalMs', 'packetSize', 'maxConcurrent'];
    pingSettings.forEach(id => {
        elements[id].addEventListener('change', async () => {
            if (state.targets.length > 0 && !state.isRunning) {
                const pingConfig = {
                    timeout_ms: parseInt(elements.timeoutMs.value) || 1000,
                    packet_size: parseInt(elements.packetSize.value) || 128,
                    interval_ms: parseInt(elements.intervalMs.value) || 1000,
                    max_concurrent: parseInt(elements.maxConcurrent.value) || 200
                };
                
                try {
                    await invoke('update_ping_config', { pingConfig });
                    showToast('设置已更新', 'info');
                } catch (err) {
                    console.error('Update config error:', err);
                }
            }
        });
    });
}

// =============================================================================
// Initialization
// =============================================================================

async function init() {
    console.log('PingTest initializing...');
    
    // Load config
    try {
        const config = await invoke('get_config');
        
        // Apply saved settings
        elements.timeoutMs.value = config.ping?.timeout_ms || 1000;
        elements.intervalMs.value = config.ping?.interval_ms || 1000;
        elements.packetSize.value = config.ping?.packet_size || 128;
        elements.maxConcurrent.value = config.ping?.max_concurrent || 200;
        elements.stripFirstLast.checked = config.cidr_strip_first_last !== false;
        
        // Restore last addresses if remember_addresses is enabled
        if (config.remember_addresses && config.last_addresses?.length > 0) {
            elements.targetInput.value = config.last_addresses.join('\n');
            await parseAndCountTargets();
        }
        
        console.log('Config loaded:', config);
    } catch (err) {
        console.error('Failed to load config:', err);
    }
    
    // Check if ping is already running (e.g., after window close/reopen)
    try {
        const running = await invoke('is_ping_running');
        if (running) {
            updateRunningStatus(true);
            state.statsInterval = setInterval(() => {
                refreshTable();
                updateStatsSummary();
            }, 1000);
        }
    } catch (err) {
        console.error('Failed to check ping status:', err);
    }
    
    // Initialize event handlers
    initEventHandlers();
    
    // Initial status
    updateConnectionStatus('disconnected');
    elements.runningStatus.textContent = '状态: 空闲';
    
    console.log('PingTest initialized');
}

// Start the app
document.addEventListener('DOMContentLoaded', init);
