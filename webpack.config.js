const path = require('path');
const wasmPlugin = require('@wasm-tool/wasm-pack-plugin');
const copyPlugin = require('copy-webpack-plugin');

const webPath = path.resolve(__dirname, 'menu');
const distPath = path.resolve(__dirname, 'build', 'dist');
module.exports = (_, argv) => {
  return {
    entry: './bootstrap.js',
    output: {
      path: distPath,
      filename: 'mineweb.js',
      webassemblyModuleFilename: 'mineweb.wasm',
    },
    module: {
      rules: [
        {
          test: /\.s[ac]ss$/i,
          use: ['style-loader', 'css-loader', 'sass-loader'],
        },
      ],
    },
    plugins: [
      new copyPlugin({ patterns: [path.resolve(webPath, 'static')] }),
      new wasmPlugin({
        crateDirectory: path.resolve(__dirname, 'menu'),
        extraArgs: '--no-typescript',
        outDir: path.resolve(__dirname, 'build', 'pkg'),
      }),
    ],
    watch: argv.mode !== 'production',
  };
};
