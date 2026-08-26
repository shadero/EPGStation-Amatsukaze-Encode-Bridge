'use strict';

const POLL_INTERVAL_MS = Number(process.env.BRIDGE_POLL_INTERVAL_MS || '10000');
const MAX_CONSECUTIVE_POLL_ERRORS = Number(process.env.BRIDGE_MAX_POLL_ERRORS || '12');

function fail(message) {
  throw new Error(message);
}

async function fetchWithTimeout(url, options = {}) {
  const response = await fetch(url, {
    ...options,
    signal: AbortSignal.timeout(30_000),
  });
  const text = await response.text();
  let body = null;
  if (text !== '') {
    try {
      body = JSON.parse(text);
    } catch {
      body = text;
    }
  }
  if (!response.ok) {
    fail(`${options.method || 'GET'} ${url} returned HTTP ${response.status}: ${text}`);
  }
  return body;
}

function sleep(milliseconds) {
  return new Promise(resolve => setTimeout(resolve, milliseconds));
}

async function main() {
  const bridgeBaseUrl = process.argv[2] || process.env.BRIDGE_URL;
  const preset = process.argv[3];
  const viewName = process.argv[4];
  const recordedIdText = process.env.RECORDEDID;
  const input = process.env.INPUT;
  const subDirectory = process.env.SUBDIR || '';

  if (!bridgeBaseUrl) {
    fail('Bridge base URL is required as the first argument or BRIDGE_URL.');
  }
  if (!preset || preset.trim() === '') {
    fail('Amatsukaze profile or auto-selection name is required as the second argument.');
  }
  if (!viewName || viewName.trim() === '') {
    fail('EPGStation view name is required as the third argument.');
  }
  const recordedId = Number(recordedIdText);
  if (!Number.isSafeInteger(recordedId) || recordedId <= 0) {
    fail(`RECORDEDID is missing or invalid: ${recordedIdText || '(missing)'}`);
  }
  if (!input) fail('INPUT is missing.');

  const normalizedInput = input.replaceAll('\\', '/');
  const inputFilename = normalizedInput.slice(normalizedInput.lastIndexOf('/') + 1);
  if (!inputFilename) fail(`Could not extract a filename from INPUT: ${input}`);

  const headers = { 'Content-Type': 'application/json; charset=utf-8' };

  const workflowsUrl = new URL('/workflows', bridgeBaseUrl).toString();
  const created = await fetchWithTimeout(workflowsUrl, {
    method: 'POST',
    headers,
    body: JSON.stringify({
      recordedId,
      inputFilename,
      preset,
      subDirectory,
      viewName,
    }),
  });
  if (!created?.workflowId || !created?.statusUrl) {
    fail(`Bridge returned an invalid workflow response: ${JSON.stringify(created)}`);
  }
  const statusUrl = new URL(created.statusUrl, bridgeBaseUrl).toString();
  console.log(
      `Bridge workflow ${created.workflowId} ${created.existing ? 'resumed' : 'accepted'}: ` +
      `recordedId=${recordedId}, preset=${preset}, ` +
      `subDirectory=${subDirectory || '(root)'}, viewName=${viewName}`,
  );

  let lastStage = '';
  let lastLogAt = 0;
  let consecutiveErrors = 0;
  for (;;) {
    await sleep(POLL_INTERVAL_MS);
    let workflow;
    try {
      workflow = await fetchWithTimeout(statusUrl, { headers });
      consecutiveErrors = 0;
    } catch (error) {
      consecutiveErrors += 1;
      console.error(
        `Bridge status check failed for workflow ${created.workflowId} ` +
        `(${consecutiveErrors}/${MAX_CONSECUTIVE_POLL_ERRORS}, url=${statusUrl}): ` +
        `${error.stack || error.message}`,
      );
      if (consecutiveErrors >= MAX_CONSECUTIVE_POLL_ERRORS) throw error;
      continue;
    }

    const now = Date.now();
    if (workflow.stage !== lastStage || now - lastLogAt >= 60_000) {
      console.log(
        `Bridge workflow ${created.workflowId}: state=${workflow.state}, stage=${workflow.stage}`,
      );
      lastStage = workflow.stage;
      lastLogAt = now;
    }

    if (workflow.state === 'succeeded') {
      console.log(`Bridge completed: output=${workflow.outputFilename}`);
      return;
    }
    if (workflow.state === 'failed') {
      fail(
        `Bridge workflow ${created.workflowId} failed: ` +
        `recordedId=${recordedId}, input=${inputFilename}, ` +
        `stage=${workflow.failedAt || workflow.stage}, ` +
        `queueItemId=${workflow.queueItemId ?? '(not submitted)'}: ` +
        `${workflow.error || 'Bridge did not return an error reason'}`,
      );
    }
    if (workflow.state !== 'queued' && workflow.state !== 'running') {
      fail(`Bridge returned an unknown workflow state: ${workflow.state}`);
    }
  }
}

main().catch(error => {
  console.error(`Amatsukaze Bridge failed: ${error.stack || error.message}`);
  process.exitCode = 1;
});
