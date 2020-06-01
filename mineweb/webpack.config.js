const path = require('path');
const WasmPackPlugin = require('@wasm-tool/wasm-pack-plugin');
const CopyWebpackPlugin = require('copy-webpack-plugin');

const distPath = path.resolve(__dirname, 'dist');
module.exports = () => {
  return {
    entry: ['./bootstrap', './sass/index.sass'],
    output: {
      path: distPath,
      filename: 'mineweb.js',
      webassemblyModuleFilename: 'mineweb.wasm',
    },
    plugins: [
      new CopyWebpackPlugin([{ from: './static', to: distPath }]),
      new WasmPackPlugin({
        crateDirectory: '.',
        extraArgs: '--no-typescript',
      }),
    ],
    watch: false,
    module: {
      rules: [
        {
          test: /\.s[ac]ss$/i,
          use: [
            {
              loader: 'file-loader',
              options: {
                name: '[name].css',
              },
            },
            {
              loader: 'extract-loader',
            },
            {
              loader: 'css-loader?-url',
            },
            {
              loader: 'sass-loader',
            },
          ],
        },
      ],
    },
  };
};
