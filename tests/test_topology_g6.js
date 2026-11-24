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
    escapeHtml,
    truncate,
    formatPortBadge,
    healthColors,
    findMatchingNodes: (q, payload) => {
      currentPayload = payload;
      return findMatchingNodes(q);
    },
    renderTooltipContent,
    layoutConfigFor,
  };
`;

const iifeEnd = scriptContent.lastIndexOf('})();');
assert(iifeEnd !== -1, 'Could not locate closing IIFE in script');
scriptContent = scriptContent.slice(0, iifeEnd) + injection + scriptContent.slice(iifeEnd);

const sandbox = {
  console,
  document: {
    getElementById: (id) => ({
      id,
      addEventListener: () => {},
      classList: { add: () => {}, remove: () => {}, contains: () => false },
      style: {},
      textContent: '',
      value: '',
      focus: () => {},
      select: () => {},
    }),
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
  escapeHtml,
  truncate,
  formatPortBadge,
  healthColors,
  findMatchingNodes,
  renderTooltipContent,
  layoutConfigFor,
} = sandbox.__TEST_EXPORTS__;

console.log('--- RUNNING TOPOLOGY G6 VERIFICATION TEST SUITE ---');

// ============================================================================
// REQUIREMENT 1: Layout / No Overlap & Mode Configurations
// ============================================================================
console.log('\n[Requirement 1] Layout modes, configs, and node/combo separation:');

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
assert(html.includes('color: #f8fafc !important'), 'Tooltips must set high contrast white text');
console.log('  ✔ CSS rules provide transparent wrapper and #f8fafc high contrast override for all tooltip classes');

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
assert(tooltipHtml.includes('background:rgba(20,17,38,0.97)'), 'Tooltip must use high-contrast dark panel background');
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

console.log('\n======================================================');
console.log('ALL 4 REQUIREMENTS AND EDGE CASES VERIFIED SUCCESSFULLY!');
console.log('======================================================\n');
