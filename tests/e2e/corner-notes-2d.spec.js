const { test, expect } = require("@playwright/test");

async function openPlayableBoard(page) {
  await page.goto("/");
  await expect(page.locator("#board .cell")).toHaveCount(20);
  const startButton = page.locator("#start-button");
  if (await startButton.isVisible()) {
    await startButton.click();
  }
}

function topNote(page) {
  return page.locator("#board .cell.hidden-clue .cell-note").first();
}

function bottomNote(page) {
  return page.locator("#board .cell.hidden-clue .cell-note-secondary").first();
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

test("top corner note supports single, double, and triple tap colors", async ({ page }) => {
  await openPlayableBoard(page);

  const note = topNote(page);

  await tapLocator(page, note, 1);
  await expect(note).toHaveClass(/note-yellow/);

  await page.waitForTimeout(500);
  await tapLocator(page, note, 1);
  await expect(note).toHaveClass(/note-none/);

  await tapLocator(page, note, 2);
  await expect(note).toHaveClass(/note-red/);

  await page.waitForTimeout(500);
  await tapLocator(page, note, 1);
  await expect(note).toHaveClass(/note-none/);

  await tapLocator(page, note, 3);
  await expect(note).toHaveClass(/note-green/);
});

test("corner note color picker applies colors to top and bottom note slots", async ({ page }) => {
  await openPlayableBoard(page);

  const top = topNote(page);
  await chooseLongPressColor(page, top, "orange");
  await expect(top).toHaveClass(/note-orange/);

  const bottom = bottomNote(page);
  await chooseLongPressColor(page, bottom, "cyan");
  await expect(bottom).toHaveClass(/note-cyan/);

  await page.waitForTimeout(500);
  await tapLocator(page, bottom, 1);
  await expect(bottom).toHaveClass(/note-none/);
});
