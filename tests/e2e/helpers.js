const { expect } = require("@playwright/test");

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

async function centerOf(locator) {
  const box = await locator.boundingBox();
  if (!box) {
    throw new Error("Target is not visible for pointer interaction.");
  }

  return {
    x: box.x + box.width / 2,
    y: box.y + box.height / 2,
  };
}

async function tapLocator(page, locator, count) {
  const point = await centerOf(locator);
  for (let index = 0; index < count; index += 1) {
    await page.mouse.click(point.x, point.y);
    if (index + 1 < count) {
      await page.waitForTimeout(60);
    }
  }
}

async function longPressLocator(page, locator) {
  const point = await centerOf(locator);
  await locator.dispatchEvent("pointerdown", {
    pointerType: "mouse",
    pointerId: 1,
    button: 0,
    buttons: 1,
    isPrimary: true,
    clientX: point.x,
    clientY: point.y,
  });
  await page.waitForTimeout(520);
}

async function chooseLongPressColor(page, locator, color) {
  await longPressLocator(page, locator);
  const menu = page.locator("#corner-note-menu");
  await expect(menu).toBeVisible();

  const option = page.locator(`#corner-note-menu [data-color="${color}"]`);
  const point = await centerOf(option);

  await page.evaluate(
    ({ x, y }) => {
      window.dispatchEvent(
        new PointerEvent("pointermove", {
          bubbles: true,
          pointerType: "mouse",
          pointerId: 1,
          button: 0,
          buttons: 1,
          isPrimary: true,
          clientX: x,
          clientY: y,
        }),
      );
    },
    { x: point.x, y: point.y },
  );

  await page.evaluate(
    ({ x, y }) => {
      window.dispatchEvent(
        new PointerEvent("pointerup", {
          bubbles: true,
          pointerType: "mouse",
          pointerId: 1,
          button: 0,
          buttons: 0,
          isPrimary: true,
          clientX: x,
          clientY: y,
        }),
      );
    },
    { x: point.x, y: point.y },
  );
}

async function start2D(page) {
  await page.goto("/");
  await expect(page.locator("#board .cell")).toHaveCount(20);
  const startButton = page.locator("#start-button");
  if (await startButton.isVisible()) {
    await startButton.click();
  }
}

async function openFreshEditor2x2(page, author = "") {
  await page.goto("/edit");
  await expect(page.getByRole("heading", { name: "Clues Editor" })).toBeVisible();
  await expect(page.locator("#rows-select")).toBeEnabled();
  await page.selectOption("#rows-select", "2");
  await page.selectOption("#cols-select", "2");
  if (author) {
    await page.fill("#author-input", author);
  }
  await page.click("#reset-editor");
  await expect(page.locator("#editor-board .editor-cell")).toHaveCount(4);
}

async function buildAndShareTiny2DPuzzle(page, author = "E2E Bot") {
  await openFreshEditor2x2(page, author);
  await page.locator("#editor-board .editor-cell").first().click();
  await expect(page.locator("#set-initial-innocent")).toBeVisible();
  await page.click("#set-initial-innocent");
  await expect(page.locator("#generate-remaining")).toBeEnabled();
  await page.locator("#generate-remaining").click({ modifiers: ["Shift"] });
  const shareButton = page.locator("#share-editor");
  await expect(shareButton).toBeVisible();
  await expect(shareButton).toBeEnabled();
  await page.click("#share-editor");
  const sharedLink = page.locator("#editor-share-link a");
  await expect(sharedLink).toBeVisible();
  const href = await sharedLink.getAttribute("href");
  const storedPuzzleId = new URL(href, "http://127.0.0.1:3000").pathname.split("/").pop();
  expect(storedPuzzleId).toMatch(/^[0-9a-f]{6}$/);
  return storedPuzzleId;
}

async function fetchOpened2DStoredPuzzle(request, storedPuzzleId) {
  const response = await request.post("/api/editor/open", {
    data: {
      source: {
        kind: "stored",
        stored_puzzle_id: storedPuzzleId,
      },
    },
  });
  expect(response.ok()).toBeTruthy();
  return response.json();
}

async function startStored2DPuzzle(page, storedPuzzleId) {
  await page.goto(`/p/${storedPuzzleId}`);
  const startButton = page.locator("#start-button");
  if (await startButton.isVisible()) {
    await startButton.click();
  }
}

async function solveStored2DPuzzle(page, request, storedPuzzleId) {
  const opened = await fetchOpened2DStoredPuzzle(request, storedPuzzleId);
  const initialReveal = opened.draft.initial_reveal;
  const steps = opened.progression.filter(
    (step) =>
      step.kind === "add_clue" &&
      !(initialReveal && step.row === initialReveal.row && step.col === initialReveal.col),
  );

  await startStored2DPuzzle(page, storedPuzzleId);

  for (const step of steps) {
    const authoredCell = opened.draft.cells[step.row][step.col];
    const alreadyRevealed = await page.evaluate(
      ({ row, col }) => state.cells?.[row]?.[col]?.revealed === true,
      { row: step.row, col: step.col },
    );
    if (alreadyRevealed) {
      continue;
    }

    const card = page
      .locator("#board .cell")
      .filter({
        has: page.locator(".cell-name", {
          hasText: new RegExp(`^${escapeRegex(authoredCell.name)}$`),
        }),
      })
      .first();

    await expect(card).toBeVisible();
    await card.click();
    await expect(page.locator("#guess-modal")).toBeVisible();
    await page.click(
      authoredCell.answer === "innocent" ? "#guess-innocent" : "#guess-criminal",
    );
    await expect(page.locator("#guess-modal")).toBeHidden();
  }

  return opened;
}

async function open3D(
  page,
  url = "/3d/?seed=921d06880b8b&depth=2&rows=2&cols=2",
  { start = true } = {},
) {
  await page.goto(url);
  await expect(page.locator("#board-3d-scene .three-d-face")).toHaveCount(48);
  const startButton = page.locator("#start-button");
  if (start && (await startButton.isVisible())) {
    await startButton.click();
  }
}

async function wheelOn(page, selector, deltaX, deltaY, ctrlKey = false) {
  await page.locator(selector).evaluate(
    (element, payload) => {
      element.dispatchEvent(
        new WheelEvent("wheel", {
          bubbles: true,
          cancelable: true,
          deltaX: payload.deltaX,
          deltaY: payload.deltaY,
          ctrlKey: payload.ctrlKey,
        }),
      );
    },
    { deltaX, deltaY, ctrlKey },
  );
}

module.exports = {
  buildAndShareTiny2DPuzzle,
  chooseLongPressColor,
  escapeRegex,
  fetchOpened2DStoredPuzzle,
  open3D,
  openFreshEditor2x2,
  solveStored2DPuzzle,
  start2D,
  startStored2DPuzzle,
  tapLocator,
  wheelOn,
};
