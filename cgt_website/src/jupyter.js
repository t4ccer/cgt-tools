// Runs a notebook in the Jupyter UI for `jupyter.rs`. Every method returns a promise of a JSON
// string, which is how the result gets back over the DevTools protocol
window.cgtRunner = (() => {
  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  async function until(what, timeout, check) {
    const end = Date.now() + timeout;
    for (;;) {
      const value = check();
      if (value) {
        return value;
      }
      if (Date.now() > end) {
        throw new Error(`timed out waiting for ${what}`);
      }
      await sleep(100);
    }
  }

  const panel = () => window.jupyterapp?.shell?.currentWidget;
  const kernelStatus = () => panel()?.sessionContext?.session?.kernel?.status;

  // A widget is drawn only after its cell has finished, and the callbacks that showing it
  // triggers run after that, so the notebook is done once the kernel has stayed idle and the page
  // has stopped changing for a while
  async function settle(what) {
    let last;
    let since = Date.now();
    await until(what, 120000, () => {
      const state = `${kernelStatus()} ${panel().content.node.innerHTML.length}`;
      if (state !== last || kernelStatus() !== "idle") {
        last = state;
        since = Date.now();
      }
      return Date.now() - since > 1500;
    });
  }

  return {
    async open() {
      const notebook = await until(
        "the notebook to open",
        120000,
        () => panel()?.content?.widgets?.length > 0 && panel(),
      );
      await notebook.sessionContext.ready;
      await until(
        "the kernel to start",
        120000,
        () => kernelStatus() === "idle",
      );
      return "null";
    },

    async runAll() {
      await window.jupyterapp.commands.execute("notebook:run-all-cells");
      await settle("the notebook to finish");
      const cells = panel().content.widgets.filter(
        (cell) => cell.model.type === "code",
      );
      for (const cell of cells) {
        const text = cell.outputArea.node.textContent;
        if (text.includes("Error displaying widget")) {
          throw new Error(`a cell could not display its widget: ${text}`);
        }
      }
      return JSON.stringify(cells.length);
    },

    async save() {
      await window.jupyterapp.commands.execute("docmanager:save");
      return "null";
    },

    // The box around what the widget output of a cell draws, in CSS pixels from the top left
    // corner of the window. Outputs stretch across the notebook, so the box is made of the
    // controls, pictures and text inside the output instead
    async measure(index) {
      const cell = panel().content.widgets[index];
      const outputs = cell.model.outputs;
      let output = null;
      for (let i = 0; i < outputs.length && !output; i++) {
        if ("application/vnd.jupyter.widget-view+json" in outputs.get(i).data) {
          output = cell.outputArea.widgets[i].node.querySelector(
            ".jp-OutputArea-output",
          );
        }
      }
      if (!output) {
        throw new Error(`cell ${index} shows no widget`);
      }
      output.scrollIntoView({ block: "start" });
      await new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      );

      const drawn = [
        "CANVAS",
        "IMG",
        "SELECT",
        "BUTTON",
        "INPUT",
        "TEXTAREA",
        "svg",
      ];
      const rects = [];
      const walker = document.createTreeWalker(
        output,
        NodeFilter.SHOW_ELEMENT | NodeFilter.SHOW_TEXT,
      );
      for (let node = walker.currentNode; node; node = walker.nextNode()) {
        if (node.nodeType === Node.TEXT_NODE) {
          if (node.textContent.trim()) {
            const range = document.createRange();
            range.selectNodeContents(node);
            rects.push(...range.getClientRects());
          }
        } else if (drawn.includes(node.tagName)) {
          rects.push(node.getBoundingClientRect());
        }
      }
      const visible = rects.filter((rect) => rect.width > 0 && rect.height > 0);
      if (visible.length === 0) {
        throw new Error(`the widget of cell ${index} draws nothing`);
      }
      const margin = 2;
      const left = Math.min(...visible.map((rect) => rect.left)) - margin;
      const top = Math.min(...visible.map((rect) => rect.top)) - margin;
      const right = Math.max(...visible.map((rect) => rect.right)) + margin;
      const bottom = Math.max(...visible.map((rect) => rect.bottom)) + margin;
      return JSON.stringify({
        x: left,
        y: top,
        width: right - left,
        height: bottom - top,
      });
    },
  };
})();
