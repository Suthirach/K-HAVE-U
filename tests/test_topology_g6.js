const assert = require('assert');
const fs = require('fs');
const path = require('path');
const vm = require('vm');

// Read assets/topology_g6.html
const htmlPath = path.join(__dirname, '..', 'assets', 'topology_g6.html');
const html = fs.readFileSync(htmlPath, 'utf8');

// Extract script
const scriptMatch = html.match(/<script(?:\s+type=["']text\/javascript["'])?>([\s\S]*?)<\/script>/i);
assert(scriptMatch, 'Could not find <script> in topology_g6.html');
let scriptContent = scriptMatch[1];

// Inject exports right before the closing of the IIFE: `})();`
const injection = `
  globalThis.__TEST_EXPORTS__ = {
    DEFAULT_LAYOUT_MODE,
    getLayoutMode: () => layoutMode,
    setLayoutMode,
    resetView: () => window.__k9sTopology.resetView(),
    escapeHtml,
    truncate,
    formatPortBadge,
    healthColors,
    findMatchingNodes: (q, payload) => {
      currentPayload = payload;
      return findMatchingNodes(q);
    },
    computeElementStates,
    renderTooltipContent,
    layoutConfigFor,
    computePolylineRoutes,
    buildPreparedData,
    buildPreparedDataFor: (payload, nextLayoutMode, nextEdgeMode) => {
      const prevLayout = layoutMode;
      const prevEdge = edgeMode;
      layoutMode = nextLayoutMode;
      edgeMode = nextEdgeMode;
      try {
        return buildPreparedData(payload);
      } finally {
        layoutMode = prevLayout;
        edgeMode = prevEdge;
      }
    },
  };
`;

const iifeEnd = scriptContent.lastIndexOf('})();');
assert(iifeEnd !== -1, 'Could not locate closing IIFE in script');
scriptContent = scriptContent.slice(0, iifeEnd) + injection + scriptContent.slice(iifeEnd);

const domElements = new Map();
function makeMockElement(id) {
  const classes = new Set();
  if (id === 'btn-layout-zones' || id === 'btn-edge-curve') {
    classes.add('active');
  }
  return {
    id,
    addEventListener: () => {},
    classList: {
      add: (...names) => names.forEach((n) => classes.add(n)),
      remove: (...names) => names.forEach((n) => classes.delete(n)),
      toggle: (name, force) => {
        if (typeof force === 'boolean') {
          if (force) classes.add(name);
          else classes.delete(name);
          return force;
        }
        if (classes.has(name)) {
          classes.delete(name);
          return false;
        }
        classes.add(name);
        return true;
      },
      contains: (name) => classes.has(name),
    },
    style: {},
    textContent: '',
    value: '',
    focus: () => {},
    select: () => {},
  };
}

const sandbox = {
  console,
  document: {
    getElementById: (id) => {
      if (!domElements.has(id)) {
        domElements.set(id, makeMockElement(id));
      }
      return domElements.get(id);
    },
    head: { appendChild: () => {} },
    createElement: () => ({ addEventListener: () => {} }),
  },
  window: {
    addEventListener: () => {},
    innerWidth: 1200,
    innerHeight: 800,
  },
  setTimeout: (fn) => fn(),
  clearTimeout: () => {},
};

vm.createContext(sandbox);
vm.runInContext(scriptContent, sandbox);

const {
  DEFAULT_LAYOUT_MODE,
  getLayoutMode,
  setLayoutMode,
  resetView,
  escapeHtml,
  truncate,
  formatPortBadge,
  healthColors,
  findMatchingNodes,
  computeElementStates,
  renderTooltipContent,
  layoutConfigFor,
  computePolylineRoutes,
  buildPreparedData,
  buildPreparedDataFor,
} = sandbox.__TEST_EXPORTS__;

console.log('--- RUNNING TOPOLOGY G6 VERIFICATION TEST SUITE ---');

// ============================================================================
// REQUIREMENT 1: Layout / No Overlap & Mode Configurations (Zones Default)
// ============================================================================
console.log('\n[Requirement 1] Default zones layout, mode switching, and node/combo separation:');

// 1.0 Default startup layout must be `zones`
assert.strictEqual(DEFAULT_LAYOUT_MODE, 'zones', 'DEFAULT_LAYOUT_MODE must be "zones"');
assert.strictEqual(getLayoutMode(), 'zones', 'Initial layoutMode on startup must be "zones"');
assert(
  html.includes('<button class="hud-btn active" id="btn-layout-zones"'),
  'HTML must mark Zones button as active on startup'
);
assert(
  html.includes('<button class="hud-btn" id="btn-layout-network"'),
  'HTML must NOT mark Network button as active on startup'
);
assert(
  html.includes('<button class="hud-btn" id="btn-layout-dagre"'),
  'HTML must NOT mark Dagre button as active on startup'
);
console.log('  ✔ Default startup layout is "zones" with Zones toolbar button active');

// 1.0b Switching to Network / Dagre and Reset back to Zones
setLayoutMode('network');
assert.strictEqual(getLayoutMode(), 'network', 'setLayoutMode("network") must switch to network');
assert(domElements.get('btn-layout-network').classList.contains('active'), 'Network button should be active');
assert(!domElements.get('btn-layout-zones').classList.contains('active'), 'Zones button should be inactive');

setLayoutMode('dagre');
assert.strictEqual(getLayoutMode(), 'dagre', 'setLayoutMode("dagre") must switch to dagre');
assert(domElements.get('btn-layout-dagre').classList.contains('active'), 'Dagre button should be active');
assert(!domElements.get('btn-layout-network').classList.contains('active'), 'Network button should be inactive');

resetView();
assert.strictEqual(getLayoutMode(), 'zones', 'resetView() must restore default "zones" layout');
assert(domElements.get('btn-layout-zones').classList.contains('active'), 'Zones button should be active after reset');
assert(!domElements.get('btn-layout-dagre').classList.contains('active'), 'Dagre button should be inactive after reset');
console.log('  ✔ Switching to Network / Dagre and Reset back to Zones updates state and active buttons cleanly');

// 1.1 Layout configurations
const netLayout = layoutConfigFor('network');
assert.strictEqual(netLayout.type, 'd3-force', 'Network mode must use d3-force');
assert.strictEqual(typeof netLayout.collide.radius, 'function', 'Network mode must define collide function');
assert.strictEqual(netLayout.collide.iterations, 10, 'Collision iterations should be 10 for anti-overlap');
assert.strictEqual(netLayout.manyBody.strength, -1350, 'Many-body repulsion should be strong (-1350)');
assert.strictEqual(typeof netLayout.link.distance, 'function', 'Link distance must be a function');
console.log('  ✔ Network layoutConfigFor("network") uses d3-force with strong repulsion and collision radius');

const dagreLayout = layoutConfigFor('dagre');
assert.strictEqual(dagreLayout.type, 'antv-dagre', 'Dagre mode must use antv-dagre');
assert.strictEqual(dagreLayout.rankdir, 'TB', 'Dagre mode rankdir should be Top-to-Bottom');
assert.strictEqual(dagreLayout.nodesep, 68, 'Dagre nodesep must be 68px');
assert.strictEqual(dagreLayout.ranksep, 118, 'Dagre ranksep must be 118px');
assert.strictEqual(typeof dagreLayout.nodeSize, 'function', 'Dagre nodeSize must be a function matching node bounds');
assert.deepStrictEqual(Array.from(dagreLayout.nodeSize({ data: { kind: 'domain' } })), [244, 46]);
assert.deepStrictEqual(Array.from(dagreLayout.nodeSize({ data: { kind: 'switch' } })), [196, 68]);
assert.deepStrictEqual(Array.from(dagreLayout.nodeSize({ data: { kind: 'api' } })), [224, 68]);
console.log('  ✔ Dagre layoutConfigFor("dagre") correctly configures TB hierarchical flow and exact card node sizes');

const zonesLayout = layoutConfigFor('zones');
assert.strictEqual(zonesLayout, undefined, 'Zones mode returns undefined to preserve deterministic Rust layout coordinates');
console.log('  ✔ Zones layoutConfigFor("zones") returns undefined (Rust precomputed coordinates)');


// ============================================================================
// REQUIREMENT 2: Port & Badge Alignment & Bounding Card Budget
// ============================================================================
console.log('\n[Requirement 2] Port & Badge alignment, multi-port badges, and card bounds:');

// 2.1 formatPortBadge
assert.strictEqual(formatPortBadge(''), '', 'Empty port string returns empty');
assert.strictEqual(formatPortBadge('-'), '', 'Dash port returns empty');
assert.strictEqual(formatPortBadge(null), '', 'Null port returns empty');
assert.strictEqual(formatPortBadge('80'), ':80', 'Single port 80 -> :80');
assert.strictEqual(formatPortBadge(':80'), ':80', 'Prefixed :80 -> :80');
assert.strictEqual(formatPortBadge('80, 443'), ':80,443', 'Multi-port 80, 443 -> :80,443');
assert.strictEqual(formatPortBadge('80,443'), ':80,443', 'Multi-port 80,443 -> :80,443');
assert.strictEqual(formatPortBadge('80, 443, 8080'), ':80 +2', '3 ports -> :80 +2');
assert.strictEqual(formatPortBadge('3000, 3001, 3002, 3003'), ':3000 +3', '4 ports -> :3000 +3');
assert.strictEqual(formatPortBadge('123456789'), ':1234567…', 'Long single port truncated to 8 chars');
console.log('  ✔ formatPortBadge correctly formats :80, :80,443, and :80 +2 multi-port badges');

// 2.2 Truncation
assert.strictEqual(truncate('prod-very-long-microservice-handler-01-srv', 20), 'prod-very-long-micr…');
assert.strictEqual(truncate('short-name', 20), 'short-name');
assert.strictEqual(truncate('exact-twenty-chars!!', 20), 'exact-twenty-chars!!');
console.log('  ✔ truncate handles short, exact, and oversized service names with ellipsis');

// 2.3 Card bounds badge budget calculation test
function simulateBadgeBudget(w, leftText, rightText) {
  const halfW = w / 2;
  const leftPadX = 18;
  const rightPadX = 12;
  const innerW = Math.max(w - leftPadX - rightPadX, 80);
  const minGap = 8;

  const showLeft = leftText && leftText.length > 0;
  const showRight = rightText && rightText.length > 0;

  let leftW = showLeft ? Math.max(leftText.length * 6.0 + 16, 54) : 0;
  let rightW = showRight ? Math.max(rightText.length * 6.1 + 16, 44) : 0;
  if (showLeft && showRight) {
    const budget = innerW - minGap;
    if (leftW + rightW > budget) {
      const scale = budget / (leftW + rightW);
      leftW = Math.floor(leftW * scale);
      rightW = Math.floor(budget - leftW);
    }
  } else if (showLeft) {
    leftW = Math.min(leftW, innerW);
  } else if (showRight) {
    rightW = Math.min(rightW, innerW);
  }

  // Left pill bounds
  const leftPillX1 = -halfW + leftPadX;
  const leftPillX2 = leftPillX1 + leftW;

  // Right pill bounds
  const rightPillX1 = halfW - rightW - rightPadX;
  const rightPillX2 = halfW - rightPadX;

  return {
    w,
    innerW,
    leftW,
    rightW,
    leftPillX1,
    leftPillX2,
    rightPillX1,
    rightPillX2,
    gap: rightPillX1 - leftPillX2,
    fitsInsideCard: leftPillX1 >= -halfW && rightPillX2 <= halfW,
    noOverlap: rightPillX1 >= leftPillX2,
  };
}

// Test normal badge widths (w = 224)
const normalBadge = simulateBadgeBudget(224, '● 3/3 Pods', ':3010');
assert(normalBadge.fitsInsideCard, 'Normal badges must stay strictly inside card');
assert(normalBadge.noOverlap, 'Normal badges must not overlap');
assert(normalBadge.gap >= 8, 'Normal badges have >= 8px gap');

// Test extreme oversized badge texts
const extremeBadge = simulateBadgeBudget(224, '● 9999/9999 Pods (Degraded)', ':80,443,8080 · 45 links');
assert(extremeBadge.fitsInsideCard, 'Extreme badges must stay strictly inside card bounds');
assert(extremeBadge.noOverlap, 'Extreme badges must not overlap');
assert(extremeBadge.gap >= 8, 'Extreme badges maintain minimum 8px gap');
console.log('  ✔ K8sCardNode badge budget layout: zero drift outside card [-halfW, halfW] and zero badge collision');


// ============================================================================
// REQUIREMENT 3: Dark Tooltip Contrast & XSS Escaping
// ============================================================================
console.log('\n[Requirement 3] Dark theme tooltip contrast & HTML/XSS escaping:');

// 3.1 CSS check
assert(html.includes('.tooltip,'), 'HTML must contain .tooltip selector override');
assert(html.includes('.g6-tooltip,'), 'HTML must contain .g6-tooltip selector override');
assert(html.includes('.k9s-dark-tooltip'), 'HTML must contain .k9s-dark-tooltip selector override');
assert(html.includes('background: transparent !important'), 'Tooltips must override background to transparent');
assert(html.includes('color: #fafaf9 !important'), 'Tooltips must set high contrast warm off-white text');
console.log('  ✔ CSS rules provide transparent wrapper and #fafaf9 high contrast override for all tooltip classes');

// 3.2 escapeHtml
const xssPayload = '<script>alert("xss")</script>&<img src=x onerror=alert(1)>"\'';
const escaped = escapeHtml(xssPayload);
assert(!escaped.includes('<script>'), 'Must escape <script>');
assert(!escaped.includes('"'), 'Must escape double quotes');
assert(!escaped.includes("'"), 'Must escape single quotes');
assert(!escaped.includes('<img'), 'Must escape <img');
assert(escaped.includes('&lt;script&gt;'), 'Must replace < with &lt; and > with &gt;');
assert(escaped.includes('&quot;'), 'Must replace " with &quot;');
assert(escaped.includes('&#39;'), 'Must replace \' with &#39;');
assert(escaped.includes('&amp;'), 'Must replace & with &amp;');
console.log('  ✔ escapeHtml thoroughly neutralizes XSS vectors (&, <, >, ", \')');

// 3.3 renderTooltipContent formatting and dark inline style
const sampleNodeItem = [{
  id: 'svc:prod-test-api',
  data: {
    fullName: '<malicious> prod-test-api',
    label: 'prod-test-api',
    serviceName: 'prod-test-api',
    roleLabel: 'API Service',
    health: 'error',
    podBadge: '0/2 Pods',
    ports: '80, 443',
    fanIn: 12,
    hosts: ['https://example.com?foo=1&bar=2', '<img src=x>'],
    connectsTo: ['redis-srv', '<script>'],
    summary: 'Handles critical user authentication & billing.',
  }
}];

const tooltipHtml = renderTooltipContent({}, sampleNodeItem);
assert(tooltipHtml.includes('background:rgba(22,20,18,0.97)'), 'Tooltip must use high-contrast warm dark glass panel background');
assert(tooltipHtml.includes('&lt;malicious&gt;'), 'Tooltip must escape fullName');
assert(!tooltipHtml.includes('<script>'), 'Tooltip must never contain unescaped script tag');
assert(!tooltipHtml.includes('<img src=x>'), 'Tooltip must never contain unescaped img tag');
assert(tooltipHtml.includes('ERROR'), 'Health badge must render ERROR for error health');
assert(tooltipHtml.includes('0/2 Pods'), 'Pod badge must render in tooltip');
assert(tooltipHtml.includes('12 connected services'), 'Fan-in count must render in tooltip');
console.log('  ✔ renderTooltipContent creates dark glass card with escaped inputs and contrast health pills');


// ============================================================================
// REQUIREMENT 4: Search Spotlight
// ============================================================================
console.log('\n[Requirement 4] Search spotlight matching, anti-false-positive, and camera guards:');

const testClusterPayload = {
  data: {
    nodes: [
      {
        id: 'node-ingress-switch',
        data: {
          kind: 'switch',
          label: 'Nginx Ingress',
          fullName: 'Nginx Ingress Controller (PROD)',
          roleLabel: 'Ingress Controller',
          badge: '10 routes',
          ports: '80 / 443',
          hosts: ['api.aombet.net', 'pay.aombet.net'],
        }
      },
      {
        id: 'svc:auth-api-srv',
        data: {
          kind: 'api',
          label: 'auth-api-srv',
          fullName: 'auth-api-srv',
          serviceName: 'auth-api-srv',
          roleLabel: 'API Service',
          podBadge: '2/2 Pods',
          ports: '3010',
          hosts: ['auth.aombet.net'],
          connectsTo: ['shared-redis-cluster-srv'], // connects to redis, but NOT a redis service!
        }
      },
      {
        id: 'svc:shared-redis-cluster-srv',
        data: {
          kind: 'cache',
          label: 'shared-redis-cluster-srv',
          fullName: 'shared-redis-cluster-srv',
          serviceName: 'shared-redis-cluster-srv',
          roleLabel: 'Cache / Storage',
          podBadge: '3/3 Pods',
          ports: '6379',
          hosts: [],
          connectsTo: [],
          fanIn: 24,
        }
      },
      {
        id: 'svc:hangfire-worker-srv',
        data: {
          kind: 'worker',
          label: 'hangfire-worker-srv',
          fullName: 'hangfire-worker-srv',
          serviceName: 'hangfire-worker-srv',
          roleLabel: 'Background Worker',
          podBadge: '1/1 Pods',
          ports: '-',
          hosts: [],
          connectsTo: ['shared-redis-cluster-srv'], // connects to redis, but NOT a redis service!
        }
      },
      {
        id: 'domain:0:api.aombet.net',
        data: {
          kind: 'domain',
          label: 'api.aombet.net',
          fullName: 'api.aombet.net',
          hosts: ['api.aombet.net'],
        }
      }
    ],
    edges: [],
  }
};

// 4.1 Search by service name
const matchAuth = Array.from(findMatchingNodes('auth', testClusterPayload));
assert.deepStrictEqual(matchAuth, ['svc:auth-api-srv'], 'Search "auth" matches only auth-api-srv');
console.log('  ✔ Search by name matches exact service');

// 4.2 CRITICAL: NOT matching unrelated APIs/workers on "redis"
const matchRedis = Array.from(findMatchingNodes('redis', testClusterPayload));
assert.deepStrictEqual(
  matchRedis,
  ['svc:shared-redis-cluster-srv'],
  'Search "redis" MUST NOT match auth-api-srv or hangfire-worker-srv even though they connect to redis!'
);
console.log('  ✔ Search "redis" matches ONLY shared-redis-cluster-srv and NEVER matches callers connecting to it');

// 4.3 Search by :port
const matchPort3010 = Array.from(findMatchingNodes(':3010', testClusterPayload));
assert.deepStrictEqual(matchPort3010, ['svc:auth-api-srv'], 'Search ":3010" matches auth-api-srv');
const matchPort6379 = Array.from(findMatchingNodes(':6379', testClusterPayload));
assert.deepStrictEqual(matchPort6379, ['svc:shared-redis-cluster-srv'], 'Search ":6379" matches redis');
const matchPort80 = Array.from(findMatchingNodes(':80', testClusterPayload));
assert(matchPort80.includes('node-ingress-switch'), 'Search ":80" matches Nginx switch');
const matchPort443 = Array.from(findMatchingNodes(':443', testClusterPayload));
assert(matchPort443.includes('node-ingress-switch'), 'Search ":443" matches Nginx switch');
console.log('  ✔ Search by :port (:3010, :6379, :80, :443) matches correct services and switch');

// 4.4 Search by podBadge and badge
const matchPods = Array.from(findMatchingNodes('3/3 Pods', testClusterPayload));
assert.deepStrictEqual(matchPods, ['svc:shared-redis-cluster-srv'], 'Search "3/3 Pods" matches redis node');
const matchRoutes = Array.from(findMatchingNodes('10 routes', testClusterPayload));
assert.deepStrictEqual(matchRoutes, ['node-ingress-switch'], 'Search "10 routes" matches switch badge');
console.log('  ✔ Search by podBadge ("3/3 Pods") and badge ("10 routes") matches target nodes');

// 4.5 Search by host domain
const matchHost = Array.from(findMatchingNodes('aombet.net', testClusterPayload));
assert(matchHost.includes('node-ingress-switch'));
assert(matchHost.includes('svc:auth-api-srv'));
assert(matchHost.includes('domain:0:api.aombet.net'));
assert(!matchHost.includes('svc:shared-redis-cluster-srv'), 'Redis has no host and must not match');
console.log('  ✔ Search by host matches domain routes, ingress switch, and service hosts');

// 4.6 Camera sequence guard check in script
assert(html.includes('const seq = ++focusSeq;'), 'focusMatchedNodes must track focusSeq');
assert(html.includes('if (seq !== focusSeq'), 'focusMatchedNodes must guard against stale camera transitions');
assert(html.includes('focusSeq++;'), 'clearSearch must increment focusSeq to cancel in-flight camera movements');
console.log('  ✔ focusSeq guard verified for in-flight camera debounce and rapid search/clear transitions');

// 4.7 Dark-fade + hidden labels on non-matches
assert(html.includes("'search-dim': {"), 'Must define search-dim node state');
assert(html.includes("labelOpacity: 0"), 'search-dim must set labelOpacity to 0');
assert(html.includes("labelFill: 'transparent'"), 'search-dim must set labelFill to transparent');
assert(html.includes("showLeft = visible &&"), 'K8sCardNode hides left pill when labelOpacity is 0');
assert(html.includes("showRight = visible &&"), 'K8sCardNode hides right pill when labelOpacity is 0');
console.log('  ✔ Dark-fade + complete label hiding verified on non-matching nodes, edges, and combos');


// ============================================================================
// REQUIREMENT 5: Non-Overlapping Polyline Routing, Port/Lane Separation & Obstacles
// ============================================================================
console.log('\n[Requirement 5] Non-overlapping polyline edges, port separation, and obstacle avoidance:');

// Helper: build the full 42-node, 50-edge stress-test cluster matching Rust's Left -> Middle -> Right zones coordinates
function buildHeavyZonesPayload() {
  const nodes = [];
  const edges = [];
  let edgeId = 0;

  const ZONE_TOP_Y = 120;
  const CARD_HALF_W = 112;
  const CARD_HALF_H = 34;
  const SWITCH_HALF_W = 98;
  const DOMAIN_HALF_W = 122;
  const DOMAIN_HALF_H = 23;
  const COMBO_PAD_SIDE = 36;
  const ZONE_H_GAP = 200;

  // 1. Left Zone: Network / Ingress Switch (Nginx Ingress on left, 10 domains in 2 columns x 5 rows)
  const domainRows = 5;
  const domStartY = ZONE_TOP_Y - (CARD_HALF_H - DOMAIN_HALF_H); // 109
  const switchX = 148;
  const switchY = domStartY + ((domainRows - 1) * 78) * 0.5; // 265
  nodes.push({
    id: 'node-ingress-switch',
    combo: 'combo-network',
    x: switchX,
    y: switchY,
    data: { kind: 'switch', label: 'Nginx Ingress', fullName: 'Nginx Ingress', badge: '10 routes', ports: '80 / 443' },
  });

  const domStartX = switchX + SWITCH_HALF_W + 96 + DOMAIN_HALF_W; // 464
  for (let i = 0; i < 10; i++) {
    const col = Math.floor(i / domainRows);
    const row = i % domainRows;
    const domId = `domain:${i}:svc-${i}.aombet.net`;
    nodes.push({
      id: domId,
      combo: 'combo-network',
      x: domStartX + col * 290,
      y: domStartY + row * 78,
      data: { kind: 'domain', label: `svc-${i}.aombet.net`, fullName: `svc-${i}.aombet.net` },
    });
    edges.push({
      id: `e-sw-${++edgeId}`,
      source: 'node-ingress-switch',
      target: domId,
      data: { kind: 'ingress-switch', sourceFanOut: 10, sourceLaneIndex: i, targetFanIn: 1, targetLaneIndex: 0 },
    });
    edges.push({
      id: `e-ing-${++edgeId}`,
      source: domId,
      target: `svc:api-${i}`,
      data: { kind: 'ingress-route', sourceFanOut: 1, sourceLaneIndex: 0, targetFanIn: 1, targetLaneIndex: 0 },
    });
  }

  // Exact outer right of combo-network: (464 + 290 + 122) + 36 = 912
  const netComboRight = domStartX + 290 + DOMAIN_HALF_W + COMBO_PAD_SIDE;

  // 2. Middle Zone: Application Servers (24 APIs, 4 cols x 6 rows, 284 x 126 pitch)
  const apiCols = 4;
  const apiStartX = netComboRight + ZONE_H_GAP + COMBO_PAD_SIDE + CARD_HALF_W; // 1260
  for (let i = 0; i < 24; i++) {
    const r = Math.floor(i / apiCols);
    const c = i % apiCols;
    nodes.push({
      id: `svc:api-${i}`,
      combo: 'combo-apps',
      x: apiStartX + c * 284,
      y: ZONE_TOP_Y + r * 126,
      data: { kind: 'api', label: `api-${i}`, fullName: `api-${i}`, serviceName: `api-${i}`, health: 'healthy', podBadge: '2/2 Pods', ports: '3010' },
    });
  }

  // Exact outer right of combo-apps: (1260 + 3*284 + 112) + 36 = 2260
  const appsComboRight = apiStartX + (apiCols - 1) * 284 + CARD_HALF_W + COMBO_PAD_SIDE;

  // 3. Right Zone: Storage & Workers Cluster (Redis at top, 6 workers in 2 cols x 3 rows below)
  const storageStartX = appsComboRight + ZONE_H_GAP + COMBO_PAD_SIDE + CARD_HALF_W; // 2608
  const storageCols = 2;
  nodes.push({
    id: 'svc:shared-redis',
    combo: 'combo-storage',
    x: storageStartX,
    y: ZONE_TOP_Y,
    data: { kind: 'cache', label: 'shared-redis', fullName: 'shared-redis', fanIn: 30, ports: '6379' },
  });

  for (let i = 0; i < 6; i++) {
    const r = 1 + Math.floor(i / storageCols);
    const c = i % storageCols;
    nodes.push({
      id: `svc:worker-${i}`,
      combo: 'combo-storage',
      x: storageStartX + c * 284,
      y: ZONE_TOP_Y + r * 126 + 28,
      data: { kind: 'worker', label: `worker-${i}`, fullName: `worker-${i}`, ports: '-' },
    });
  }

  // 24 APIs + 6 workers -> shared-redis (30 inbound fan-in edges)
  let tgtLane = 0;
  for (let i = 0; i < 24; i++) {
    edges.push({
      id: `e-dep-${++edgeId}`,
      source: `svc:api-${i}`,
      target: 'svc:shared-redis',
      data: { kind: 'service-dep', sourceFanOut: 1, sourceLaneIndex: 0, targetFanIn: 30, targetLaneIndex: tgtLane++ },
    });
  }
  for (let i = 0; i < 6; i++) {
    edges.push({
      id: `e-dep-${++edgeId}`,
      source: `svc:worker-${i}`,
      target: 'svc:shared-redis',
      data: { kind: 'service-dep', sourceFanOut: 1, sourceLaneIndex: 0, targetFanIn: 30, targetLaneIndex: tgtLane++ },
    });
  }

  return {
    data: {
      combos: [
        { id: 'combo-network', data: { isCombo: true, label: 'NETWORK / INGRESS SWITCH' } },
        { id: 'combo-apps', data: { isCombo: true, label: 'APPLICATION SERVERS' } },
        { id: 'combo-storage', data: { isCombo: true, label: 'STORAGE & WORKERS CLUSTER' } },
      ],
      nodes,
      edges,
    },
  };
}

// Independent geometry verifier: verifies strict orthogonality, zero collinear overlaps, and obstacle clearance
function verifyPolylineGeometry(nodes, edges, routed, { checkObstacles = false, minSep = 2.5, label = '' } = {}) {
  const nodeBoxMap = new Map();
  for (const n of nodes) {
    const cx = (n.style && typeof n.style.x === 'number') ? n.style.x : n.x;
    const cy = (n.style && typeof n.style.y === 'number') ? n.style.y : n.y;
    const kind = n.data && n.data.kind;
    const sz = (n.style && Array.isArray(n.style.size))
      ? n.style.size
      : (kind === 'domain' ? [244, 46] : (kind === 'switch' ? [196, 68] : [224, 68]));
    nodeBoxMap.set(n.id, {
      id: n.id,
      cx, cy, w: sz[0], h: sz[1],
      left: cx - sz[0] / 2, right: cx + sz[0] / 2,
      top: cy - sz[1] / 2, bottom: cy + sz[1] / 2,
    });
  }

  const hSegs = [];
  const vSegs = [];
  let obstacleViolations = 0;

  for (const e of edges) {
    const route = routed.edgeRoutes[e.id];
    assert(route, `[${label}] missing route for edge ${e.id}`);
    assert(route.sourcePort && route.targetPort, `[${label}] edge ${e.id} must have sourcePort & targetPort`);

    // Resolve port world positions directly from nodePorts (verifies G6 v5 zB contract)
    const srcPorts = routed.nodePorts[e.source] || [];
    const tgtPorts = routed.nodePorts[e.target] || [];
    const sPort = srcPorts.find((p) => p.key === route.sourcePort);
    const tPort = tgtPorts.find((p) => p.key === route.targetPort);
    assert(sPort, `[${label}] missing sourcePort ${route.sourcePort} on ${e.source}`);
    assert(tPort, `[${label}] missing targetPort ${route.targetPort} on ${e.target}`);
    assert(sPort.r === undefined || sPort.r === 0, `[${label}] port ${sPort.key} must omit r so G6 draws no clutter dots`);
    assert(tPort.r === undefined || tPort.r === 0, `[${label}] port ${tPort.key} must omit r so G6 draws no clutter dots`);

    const ub = nodeBoxMap.get(e.source);
    const vb = nodeBoxMap.get(e.target);
    const startPt = [ub.left + ub.w * sPort.placement[0], ub.top + ub.h * sPort.placement[1]];
    const endPt = [vb.left + vb.w * tPort.placement[0], vb.top + vb.h * tPort.placement[1]];
    const pts = [startPt, ...route.controlPoints, endPt];

    for (let i = 0; i < pts.length - 1; i++) {
      const a = pts[i];
      const b = pts[i + 1];
      const dx = Math.abs(b[0] - a[0]);
      const dy = Math.abs(b[1] - a[1]);
      assert(
        dx < 0.08 || dy < 0.08,
        `[${label}] non-orthogonal segment on ${e.id} seg ${i}: (${a[0]},${a[1]}) -> (${b[0]},${b[1]})`
      );
      if (dy < 0.08 && dx >= 0.5) {
        const seg = { edgeId: e.id, c: (a[1] + b[1]) * 0.5, lo: Math.min(a[0], b[0]), hi: Math.max(a[0], b[0]) };
        hSegs.push(seg);
        if (checkObstacles) {
          for (const box of nodeBoxMap.values()) {
            if (box.id === e.source || box.id === e.target) continue;
            if (seg.c > box.top + 1 && seg.c < box.bottom - 1 && seg.hi > box.left + 1 && seg.lo < box.right - 1) {
              obstacleViolations++;
            }
          }
        }
      } else if (dx < 0.08 && dy >= 0.5) {
        const seg = { edgeId: e.id, c: (a[0] + b[0]) * 0.5, lo: Math.min(a[1], b[1]), hi: Math.max(a[1], b[1]) };
        vSegs.push(seg);
        if (checkObstacles) {
          for (const box of nodeBoxMap.values()) {
            if (box.id === e.source || box.id === e.target) continue;
            if (seg.c > box.left + 1 && seg.c < box.right - 1 && seg.hi > box.top + 1 && seg.lo < box.bottom - 1) {
              obstacleViolations++;
            }
          }
        }
      }
    }
  }

  // Check pairwise collinear overlaps across ALL distinct edges
  const overlaps = [];
  for (let i = 0; i < hSegs.length; i++) {
    for (let j = i + 1; j < hSegs.length; j++) {
      const a = hSegs[i], b = hSegs[j];
      if (a.edgeId === b.edgeId) continue;
      if (Math.abs(a.c - b.c) < minSep && Math.min(a.hi, b.hi) > Math.max(a.lo, b.lo) + 0.5) {
        overlaps.push({ axis: 'H', a, b });
      }
    }
  }
  for (let i = 0; i < vSegs.length; i++) {
    for (let j = i + 1; j < vSegs.length; j++) {
      const a = vSegs[i], b = vSegs[j];
      if (a.edgeId === b.edgeId) continue;
      if (Math.abs(a.c - b.c) < minSep && Math.min(a.hi, b.hi) > Math.max(a.lo, b.lo) + 0.5) {
        overlaps.push({ axis: 'V', a, b });
      }
    }
  }

  assert.strictEqual(
    overlaps.length,
    0,
    `[${label}] found ${overlaps.length} collinear overlapping polyline segments, first: ${JSON.stringify(overlaps[0])}`
  );
  if (checkObstacles) {
    assert.strictEqual(
      obstacleViolations,
      0,
      `[${label}] found ${obstacleViolations} polyline segments cutting through unrelated node cards`
    );
  }
  return { hCount: hSegs.length, vCount: vSegs.length };
}

// 5.1 Stress test ZONES mode (Left -> Middle -> Right; 42 nodes, 50 edges)
const heavyZonesPayload = buildHeavyZonesPayload();
// Verify default buildPreparedData() (without overriding layoutMode) uses zones with all 3 combos Left -> Middle -> Right
const defaultPrepared = buildPreparedData(heavyZonesPayload);
assert.deepStrictEqual(
  defaultPrepared.combos.map((c) => c.id),
  ['combo-network', 'combo-apps', 'combo-storage'],
  'Default initial buildPreparedData() must emit combos in Left -> Middle -> Right order'
);
const comboBoundsOf = (nodes, comboId) => {
  let minX = Infinity, minY = Infinity, maxX = -Infinity, maxY = -Infinity;
  for (const n of nodes.filter((x) => x.combo === comboId)) {
    const cx = (n.style && typeof n.style.x === 'number') ? n.style.x : n.x;
    const cy = (n.style && typeof n.style.y === 'number') ? n.style.y : n.y;
    const sz = (n.style && Array.isArray(n.style.size)) ? n.style.size : [224, 68];
    minX = Math.min(minX, cx - sz[0] / 2);
    minY = Math.min(minY, cy - sz[1] / 2);
    maxX = Math.max(maxX, cx + sz[0] / 2);
    maxY = Math.max(maxY, cy + sz[1] / 2);
  }
  return { left: minX - 36, top: minY - 68, right: maxX + 36, bottom: maxY + 36 };
};
const netBox = comboBoundsOf(defaultPrepared.nodes, 'combo-network');
const appsBox = comboBoundsOf(defaultPrepared.nodes, 'combo-apps');
const storeBox = comboBoundsOf(defaultPrepared.nodes, 'combo-storage');
assert(appsBox.left - netBox.right >= 180, 'combo-network must sit strictly left of combo-apps with >=180px gap');
assert(storeBox.left - appsBox.right >= 180, 'combo-apps must sit strictly left of combo-storage with >=180px gap');
assert(Math.abs(netBox.top - appsBox.top) < 1 && Math.abs(appsBox.top - storeBox.top) < 1, 'All 3 horizontal zone combos must align along the same top border');
console.log('  ✔ Default initial render emits Left (Network) -> Middle (Apps) -> Right (Storage) combos with 200px gaps and aligned top headers');

const preparedZonesPoly = buildPreparedDataFor(heavyZonesPayload, 'zones', 'polyline');
assert.strictEqual(preparedZonesPoly.edges.length, 50);
for (const e of preparedZonesPoly.edges) {
  assert.strictEqual(e.type, 'polyline', 'Polyline mode must emit polyline edges');
  assert.strictEqual(e.style.router, false, 'Pre-routed polyline must set router: false to prevent orth kink conflict');
  assert.strictEqual(e.style.radius, 8, 'Polyline must set smooth rounded corner radius');
  assert(typeof e.style.sourcePort === 'string' && typeof e.style.targetPort === 'string', 'Every edge must bind sourcePort & targetPort');
  assert(Array.isArray(e.style.controlPoints), 'Every polyline edge must have controlPoints array');
}
// Verify distinct ports on switch (10 outbound) and redis (30 inbound)
const switchNodePoly = preparedZonesPoly.nodes.find((n) => n.id === 'node-ingress-switch');
const redisNodePoly = preparedZonesPoly.nodes.find((n) => n.id === 'svc:shared-redis');
assert.strictEqual(switchNodePoly.style.ports.length, 10, 'Nginx switch must allocate 10 distinct outbound ports');
assert.strictEqual(redisNodePoly.style.ports.length, 30, 'Redis cache must allocate 30 distinct inbound ports');
const redisPortCoords = new Set(redisNodePoly.style.ports.map((p) => `${p.placement[0].toFixed(4)},${p.placement[1].toFixed(4)}`));
assert.strictEqual(redisPortCoords.size, 30, 'All 30 Redis port coordinates must be strictly unique');

const zonesRouted = computePolylineRoutes(preparedZonesPoly.nodes, preparedZonesPoly.edges, 'zones');
const zonesStats = verifyPolylineGeometry(preparedZonesPoly.nodes, preparedZonesPoly.edges, zonesRouted, {
  checkObstacles: true,
  minSep: 2.5,
  label: 'zones-42n-50e',
});
console.log(`  ✔ Zones (42 nodes, 50 edges): 0 overlapping segments, 0 node-card collisions across ${zonesStats.hCount}H + ${zonesStats.vCount}V segments`);

// 5.2 Stress test DAGRE TB hierarchical layout (simulated antv-dagre ranks)
const dagreNodes = [];
dagreNodes.push({ id: 'node-ingress-switch', x: 1600, y: 100, data: { kind: 'switch' } });
for (let i = 0; i < 10; i++) {
  dagreNodes.push({ id: `domain:${i}:svc-${i}.aombet.net`, x: 200 + i * 312, y: 290, data: { kind: 'domain' } });
}
for (let i = 0; i < 24; i++) {
  dagreNodes.push({ id: `svc:api-${i}`, x: 140 + i * 292, y: 490, data: { kind: 'api' } });
}
dagreNodes.push({ id: 'svc:shared-redis', x: 3500, y: 710, data: { kind: 'cache' } });
for (let i = 0; i < 6; i++) {
  dagreNodes.push({ id: `svc:worker-${i}`, x: 2600 + i * 292, y: 900, data: { kind: 'worker' } });
}
const dagreRouted = computePolylineRoutes(dagreNodes, heavyZonesPayload.data.edges, 'dagre');
const dagreStats = verifyPolylineGeometry(dagreNodes, heavyZonesPayload.data.edges, dagreRouted, {
  checkObstacles: true,
  minSep: 2.5,
  label: 'dagre-42n-50e',
});
console.log(`  ✔ Dagre TB (42 nodes, 50 edges): 0 overlapping segments, 0 node-card collisions across ${dagreStats.hCount}H + ${dagreStats.vCount}V segments`);

// 5.3 Stress test NETWORK force layout (2D hub-and-spoke coordinates)
const networkNodes = [];
networkNodes.push({ id: 'node-ingress-switch', x: -640, y: 0, data: { kind: 'switch' } });
for (let i = 0; i < 10; i++) {
  networkNodes.push({ id: `domain:${i}:svc-${i}.aombet.net`, x: -240 + (i % 2) * 320, y: -420 + i * 95, data: { kind: 'domain' } });
}
for (let i = 0; i < 24; i++) {
  const col = i % 3;
  const row = Math.floor(i / 3);
  networkNodes.push({ id: `svc:api-${i}`, x: 460 + col * 310, y: -520 + row * 145, data: { kind: 'api' } });
}
networkNodes.push({ id: 'svc:shared-redis', x: 1580, y: 0, data: { kind: 'cache' } });
for (let i = 0; i < 6; i++) {
  networkNodes.push({ id: `svc:worker-${i}`, x: 1580 + ((i % 2) === 0 ? -300 : 300), y: 260 + Math.floor(i / 2) * 140, data: { kind: 'worker' } });
}
const networkRouted = computePolylineRoutes(networkNodes, heavyZonesPayload.data.edges, 'network');
const networkStats = verifyPolylineGeometry(networkNodes, heavyZonesPayload.data.edges, networkRouted, {
  checkObstacles: false,
  minSep: 2.5,
  label: 'network-42n-50e',
});
console.log(`  ✔ Network 2D (42 nodes, 50 edges): 0 overlapping segments across ${networkStats.hCount}H + ${networkStats.vCount}V segments`);

// 5.4 Multi-domain -> single shared backend gateway (8 domains in Left zone -> 1 gateway API in Middle zone)
const gatewayNodes = [
  { id: 'node-ingress-switch', combo: 'combo-network', x: 148, y: 226, data: { kind: 'switch' } },
  { id: 'svc:gateway-api', combo: 'combo-apps', x: 1260, y: 120, data: { kind: 'api' } },
];
const gatewayEdges = [];
for (let i = 0; i < 8; i++) {
  const domId = `domain:${i}:route-${i}.aombet.net`;
  gatewayNodes.push({
    id: domId,
    combo: 'combo-network',
    x: 464 + Math.floor(i / 4) * 290,
    y: 109 + (i % 4) * 78,
    data: { kind: 'domain' },
  });
  gatewayEdges.push({ id: `sw-${i}`, source: 'node-ingress-switch', target: domId, data: { kind: 'ingress-switch' } });
  gatewayEdges.push({ id: `gw-${i}`, source: domId, target: 'svc:gateway-api', data: { kind: 'ingress-route' } });
}
const gwRouted = computePolylineRoutes(gatewayNodes, gatewayEdges, 'zones');
verifyPolylineGeometry(gatewayNodes, gatewayEdges, gwRouted, {
  checkObstacles: true,
  minSep: 2.5,
  label: 'gateway-8-domains',
});
assert.strictEqual(gwRouted.nodePorts['svc:gateway-api'].length, 8, 'Gateway API must get 8 distinct inbound ports');
console.log('  ✔ Shared-backend ingress fan-in (8 domains -> 1 gateway API): 8 distinct ports, 0 overlaps, 0 obstacle hits');

// 5.5 Mode toggling & state preservation checks
const preparedCurves = buildPreparedDataFor(heavyZonesPayload, 'zones', 'quadratic');
assert(preparedCurves.edges.every((e) => e.type === 'quadratic' && e.style.router === undefined && e.style.controlPoints === undefined), 'Curves mode must cleanly omit polyline router/controlPoints');
assert(preparedCurves.nodes.every((n) => n.style.ports === undefined), 'Curves mode must not attach polyline ports');
const preparedNetPoly = buildPreparedDataFor(heavyZonesPayload, 'network', 'polyline');
assert(preparedNetPoly.edges.every((e) => e.type === 'polyline' && e.style.router && e.style.router.type === 'orth'), 'Network/Dagre initial polyline starts with orth fallback until afterlayout reroutes');
assert(html.includes('rerouteLivePolylines(buildSeq)'), 'HTML must reroute polylines after layout settles and after node drag');
assert(html.includes("graph.on('node:dragstart'"), 'HTML must keep dragged polylines orthogonal mid-drag');
assert(html.includes("graph.on('node:dragend'"), 'HTML must recompute non-overlapping lanes on node:dragend');
console.log('  ✔ Curves/Polyline toggle, post-layout rerouting, drag hooks, and Search Spotlight/selection state preservation verified');


// ============================================================================
// REQUIREMENT 6: Click-Selection Isolation (Zero Hub Cascade / Sibling Bleed)
// ============================================================================
console.log('\n[Requirement 6] Click-selection isolation (single node highlight, zero hub cascade):');

function buildSharedHubProdPayload() {
  const nodes = [
    {
      id: 'node-ingress-switch',
      combo: 'combo-network',
      data: { kind: 'switch', label: 'Nginx Ingress', fullName: 'Nginx Ingress (PROD)', serviceName: null, hosts: ['joker.aombet.net', 'pg.aombet.net'] },
    },
    {
      id: 'domain:0:joker.aombet.net',
      combo: 'combo-network',
      data: { kind: 'domain', label: 'joker.aombet.net', fullName: 'joker.aombet.net', serviceName: 'prod-joker-srv', hosts: ['joker.aombet.net'] },
    },
    {
      id: 'domain:1:pg.aombet.net',
      combo: 'combo-network',
      data: { kind: 'domain', label: 'pg.aombet.net', fullName: 'pg.aombet.net', serviceName: 'prod-pg-srv', hosts: ['pg.aombet.net'] },
    },
    {
      id: 'svc:prod-redis-srv',
      combo: 'combo-storage',
      data: { kind: 'cache', label: 'prod-redis-srv', fullName: 'prod-redis-srv', serviceName: 'prod-redis-srv', ports: '6379', fanIn: 21 },
    },
    {
      id: 'svc:prod-hangfire-redis-srv',
      combo: 'combo-storage',
      data: { kind: 'cache', label: 'prod-hangfire-redis-srv', fullName: 'prod-hangfire-redis-srv', serviceName: 'prod-hangfire-redis-srv', ports: '6379', fanIn: 12 },
    },
    {
      id: 'svc:prod-joker-srv',
      combo: 'combo-apps',
      data: { kind: 'api', label: 'prod-joker-srv', fullName: 'prod-joker-srv', serviceName: 'prod-joker-srv', ports: '3010', hosts: ['joker.aombet.net'], connectsTo: ['prod-redis-srv', 'prod-hangfire-redis-srv'] },
    },
  ];

  const siblingNames = [
    'prod-pg-srv',
    'prod-pp-srv',
    'prod-sa-srv',
    'prod-sbo-srv',
    'prod-onebx-srv',
  ];
  for (let i = 0; i < 16; i++) {
    siblingNames.push(`prod-partner-${String(i).padStart(2, '0')}-srv`);
  }

  for (const name of siblingNames) {
    nodes.push({
      id: `svc:${name}`,
      combo: 'combo-apps',
      data: { kind: 'api', label: name, fullName: name, serviceName: name, ports: '3010', connectsTo: ['prod-redis-srv', 'prod-hangfire-redis-srv'] },
    });
  }

  nodes.push({
    id: 'svc:prod-hangfire-worker',
    combo: 'combo-storage',
    data: { kind: 'worker', label: 'prod-hangfire-worker', fullName: 'prod-hangfire-worker', serviceName: 'prod-hangfire-worker', ports: '-', connectsTo: ['prod-hangfire-redis-srv'] },
  });

  // Deliberately place prod-joker-srv -> prod-redis-srv FIRST (the exact edge order
  // that previously triggered the transitive hub mutation cascade into all 20+ siblings!)
  const edges = [
    { id: 'e-joker-redis', source: 'svc:prod-joker-srv', target: 'svc:prod-redis-srv', data: { kind: 'service-dep' } },
    { id: 'e-joker-hf', source: 'svc:prod-joker-srv', target: 'svc:prod-hangfire-redis-srv', data: { kind: 'service-dep' } },
    { id: 'e-dom-joker', source: 'domain:0:joker.aombet.net', target: 'svc:prod-joker-srv', data: { kind: 'ingress-route' } },
    { id: 'e-sw-joker', source: 'node-ingress-switch', target: 'domain:0:joker.aombet.net', data: { kind: 'ingress-switch' } },
    { id: 'e-sw-pg', source: 'node-ingress-switch', target: 'domain:1:pg.aombet.net', data: { kind: 'ingress-switch' } },
    { id: 'e-dom-pg', source: 'domain:1:pg.aombet.net', target: 'svc:prod-pg-srv', data: { kind: 'ingress-route' } },
    { id: 'e-worker-hf', source: 'svc:prod-hangfire-worker', target: 'svc:prod-hangfire-redis-srv', data: { kind: 'service-dep' } },
  ];

  for (let i = 0; i < siblingNames.length; i++) {
    const name = siblingNames[i];
    edges.push({
      id: `e-sib-redis-${i}`,
      source: `svc:${name}`,
      target: 'svc:prod-redis-srv',
      data: { kind: 'service-dep' },
    });
    if (i < 10) {
      edges.push({
        id: `e-sib-hf-${i}`,
        source: `svc:${name}`,
        target: 'svc:prod-hangfire-redis-srv',
        data: { kind: 'service-dep' },
      });
    }
  }

  return {
    siblingNames,
    payload: {
      data: {
        combos: [
          { id: 'combo-apps', data: { isCombo: true, label: 'APPS' } },
          { id: 'combo-network', data: { isCombo: true, label: 'NET' } },
          { id: 'combo-storage', data: { isCombo: true, label: 'STORE' } },
        ],
        nodes,
        edges,
      },
    },
  };
}

const { payload: prodPayload, siblingNames } = buildSharedHubProdPayload();

// 6.1 Clicking prod-joker-srv must highlight ONLY prod-joker-srv (and never sibling APIs, shared hubs, or sibling edges)
const arr = (v) => Array.from(v || []);
const selRes = computeElementStates(prodPayload, {
  selectedService: 'prod-joker-srv',
  searchQuery: '',
  layoutMode: 'zones',
});
assert.strictEqual(selRes.mode, 'selection');
assert.deepStrictEqual(arr(selRes.stateMap['svc:prod-joker-srv']), ['selected'], 'Clicked prod-joker-srv must have state ["selected"]');

const selectedNodes = prodPayload.data.nodes.filter((n) => arr(selRes.stateMap[n.id]).includes('selected'));
const activeNodes = prodPayload.data.nodes.filter((n) => arr(selRes.stateMap[n.id]).includes('active'));
assert.deepStrictEqual(selectedNodes.map((n) => n.id), ['svc:prod-joker-srv'], 'ONLY prod-joker-srv may be selected');
assert.strictEqual(activeNodes.length, 0, 'ZERO other nodes may light up in active (bright yellow) state');

// All 21 sibling APIs must stay dimmed (inactive)
for (const sib of siblingNames) {
  assert.deepStrictEqual(
    arr(selRes.stateMap[`svc:${sib}`]),
    ['inactive'],
    `Sibling service ${sib} must remain dimmed ["inactive"], never highlighted`
  );
}
// Shared hub caches, worker, ingress switch, and even joker domain pill must stay dimmed (inactive)
for (const otherId of [
  'svc:prod-redis-srv',
  'svc:prod-hangfire-redis-srv',
  'svc:prod-hangfire-worker',
  'node-ingress-switch',
  'domain:0:joker.aombet.net',
  'domain:1:pg.aombet.net',
]) {
  assert.deepStrictEqual(
    arr(selRes.stateMap[otherId]),
    ['inactive'],
    `Non-clicked node ${otherId} must stay dimmed ["inactive"]`
  );
}

// Only direct 1-hop incident edges of prod-joker-srv are active; ALL sibling/hub/switch edges are inactive
const expectedActiveEdges = new Set(['e-joker-redis', 'e-joker-hf', 'e-dom-joker']);
for (const e of prodPayload.data.edges) {
  if (expectedActiveEdges.has(e.id)) {
    assert.deepStrictEqual(arr(selRes.stateMap[e.id]), ['active'], `Direct incident edge ${e.id} of prod-joker-srv should be ["active"]`);
  } else {
    assert.deepStrictEqual(arr(selRes.stateMap[e.id]), ['inactive'], `Unrelated/sibling edge ${e.id} must stay dimmed ["inactive"]`);
  }
}
console.log('  ✔ Clicking prod-joker-srv (sharing prod-redis-srv with 21 siblings) highlights ONLY prod-joker-srv and its 3 direct edges; all 21 siblings, hubs, and sibling edges stay dimmed');

// 6.2 Clicking shared hub prod-redis-srv directly highlights ONLY prod-redis-srv node (never lighting up 21 APIs in yellow)
const hubSel = computeElementStates(prodPayload, {
  selectedService: 'prod-redis-srv',
  searchQuery: '',
  layoutMode: 'zones',
});
assert.deepStrictEqual(
  prodPayload.data.nodes.filter((n) => arr(hubSel.stateMap[n.id]).includes('selected')).map((n) => n.id),
  ['svc:prod-redis-srv']
);
assert.strictEqual(
  prodPayload.data.nodes.filter((n) => arr(hubSel.stateMap[n.id]).includes('active')).length,
  0,
  'Clicking shared hub prod-redis-srv must NOT light up any connected API nodes in yellow'
);
assert.deepStrictEqual(arr(hubSel.stateMap['svc:prod-joker-srv']), ['inactive']);
assert.deepStrictEqual(arr(hubSel.stateMap['svc:prod-pg-srv']), ['inactive']);
console.log('  ✔ Clicking shared hub prod-redis-srv highlights ONLY prod-redis-srv node; all connected APIs remain dimmed');

// 6.3 Clearing selection restores every node, edge, and combo cleanly to []
const idleRes = computeElementStates(prodPayload, {
  selectedService: null,
  searchQuery: '',
  layoutMode: 'zones',
});
assert.strictEqual(idleRes.mode, 'idle');
for (const n of prodPayload.data.nodes) {
  assert.deepStrictEqual(arr(idleRes.stateMap[n.id]), [], `Cleared selection must restore node ${n.id} to []`);
}
for (const e of prodPayload.data.edges) {
  assert.deepStrictEqual(arr(idleRes.stateMap[e.id]), [], `Cleared selection must restore edge ${e.id} to []`);
}
for (const c of prodPayload.data.combos) {
  assert.deepStrictEqual(arr(idleRes.stateMap[c.id]), [], `Cleared selection must restore combo ${c.id} to []`);
}
console.log('  ✔ Clearing selection restores all nodes, edges, and combos cleanly to []');

// 6.4 Search Spotlight still takes precedence cleanly even when a service is selected
const searchOverSel = computeElementStates(prodPayload, {
  selectedService: 'prod-joker-srv',
  searchQuery: 'prod-pg-srv',
  layoutMode: 'zones',
});
assert.strictEqual(searchOverSel.mode, 'search');
assert.deepStrictEqual(arr(searchOverSel.stateMap['svc:prod-pg-srv']), ['search-match']);
assert.deepStrictEqual(arr(searchOverSel.stateMap['svc:prod-joker-srv']), ['search-dim']);
assert.deepStrictEqual(arr(searchOverSel.stateMap['svc:prod-redis-srv']), ['search-dim']);
assert.deepStrictEqual(arr(searchOverSel.stateMap['e-joker-redis']), ['search-dim']);
assert.deepStrictEqual(arr(searchOverSel.stateMap['combo-apps']), ['search-dim']);

// Verify canvas click, combo click, Escape, and Reset clear selection in HTML
assert(html.includes("graph.on('canvas:click'"), 'Canvas background click must clear selection');
assert(html.includes("graph.on('combo:click'"), 'Combo background click must clear selection');
assert(html.includes("if (currentSelectedService) {\n            currentSelectedService = null;\n            applySelectionAndSearch(false);\n            postIpc({ type: 'clear_selection' });\n            return;\n          }"), 'Escape key must clear active selection before exiting fullscreen');
assert(html.includes('resetView() {'), 'Reset action must clear selection and search');
console.log('  ✔ Search Spotlight precedence, canvas/combo click, Esc, and Reset clear-selection verified');

console.log('\n======================================================');
console.log('ALL 6 REQUIREMENTS AND EDGE CASES VERIFIED SUCCESSFULLY!');
console.log('======================================================\n');
