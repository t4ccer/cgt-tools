const path = require("path");

module.exports = {
  entry: "./index.js",
  mode: "production",
  output: {
    filename: "bundle.js", // The single output file
    path: path.resolve(__dirname, "dist"),
    clean: true,
    library: {
      name: "JupyterCGT",
      type: "umd",
    },
    globalObject: "globalThis",
  },
  experiments: {
    asyncWebAssembly: false,
  },
  optimization: {
    minimize: true,
    splitChunks: false,
  },
  module: {
    rules: [
      {
        test: /\.wasm$/,
        type: "asset/inline",
      },
    ],
  },
};
