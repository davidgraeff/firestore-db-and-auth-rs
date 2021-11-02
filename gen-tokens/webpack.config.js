const { CleanWebpackPlugin } = require('clean-webpack-plugin')
const CopyWebpackPlugin = require('copy-webpack-plugin')
const HtmlWebpackPlugin = require('html-webpack-plugin')
const webpack = require('webpack')
const path = require('path')

module.exports = {
  // Where webpack looks to start building the bundle
  entry: ['./src/index.js'],

  // Where webpack outputs the assets and bundles
  output: {
    path: path.resolve(__dirname, './dist'),
    filename: '[name].bundle.js',
    publicPath: '/',
  },

  mode: 'development',

  // Control how source maps are generated
  devtool: 'inline-source-map',

  watch: false,

  // Spin up a server for quick development
  devServer: {
    historyApiFallback: true,
    static: "/",
    open: true,
    compress: true,
    hot: true,
    port: 8080,
  },

  // Customize the webpack build process
  plugins: [
    // Removes/cleans build folders and unused assets when rebuilding
    new CleanWebpackPlugin(),

    // Generates an HTML file from a template
    // Generates deprecation warning: https://github.com/jantimon/html-webpack-plugin/issues/1501
    new HtmlWebpackPlugin({
      title: 'webpack Boilerplate',
      template: './src/index.html', // template file
      filename: 'index.html', // output file
    }),

    new webpack.HotModuleReplacementPlugin(),
  ],

  // Determine how modules within the project are treated
  module: {
    rules: [
      // JavaScript: Use Babel to transpile JavaScript files
      { test: /\.js$/, use: ['babel-loader'] },

      // Images: Copy image files to build folder
      { test: /\.(?:ico|gif|png|jpg|jpeg)$/i, type: 'asset/resource' },

      // Fonts and SVGs: Inline files
      { test: /\.(woff(2)?|eot|ttf|otf|svg|)$/, type: 'asset/inline' },
    ],
  },

  resolve: {
    modules: ['./node_modules'],
    extensions: ['.js', '.jsx', '.json'],
  },
}