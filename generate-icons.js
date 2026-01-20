const sharp = require('sharp');
const fs = require('fs');
const path = require('path');

const iconsDir = path.join(__dirname, 'src-tauri', 'icons');

// Create a simple gradient icon
async function generateIcons() {
  // Create SVG content
  const svgContent = `
    <svg width="256" height="256" xmlns="http://www.w3.org/2000/svg">
      <defs>
        <linearGradient id="grad" x1="0%" y1="0%" x2="100%" y2="100%">
          <stop offset="0%" style="stop-color:#e94560;stop-opacity:1" />
          <stop offset="100%" style="stop-color:#ff8a80;stop-opacity:1" />
        </linearGradient>
      </defs>
      <rect width="256" height="256" rx="40" fill="url(#grad)"/>
      <text x="128" y="160" font-family="Arial, sans-serif" font-size="120" font-weight="bold" fill="white" text-anchor="middle">C</text>
    </svg>
  `;

  const svgBuffer = Buffer.from(svgContent);

  // Generate different sizes
  const sizes = [32, 128, 256];

  for (const size of sizes) {
    const filename = size === 256 ? '128x128@2x.png' : `${size}x${size}.png`;
    await sharp(svgBuffer)
      .resize(size, size)
      .png()
      .toFile(path.join(iconsDir, filename));
    console.log(`Generated ${filename}`);
  }

  // Generate ICO file (using 256x256 as base)
  const pngBuffer = await sharp(svgBuffer).resize(256, 256).png().toBuffer();

  // Simple ICO header for single 256x256 PNG
  const icoHeader = Buffer.alloc(6);
  icoHeader.writeUInt16LE(0, 0); // Reserved
  icoHeader.writeUInt16LE(1, 2); // ICO type
  icoHeader.writeUInt16LE(1, 4); // Number of images

  const icoEntry = Buffer.alloc(16);
  icoEntry.writeUInt8(0, 0);  // Width (0 = 256)
  icoEntry.writeUInt8(0, 1);  // Height (0 = 256)
  icoEntry.writeUInt8(0, 2);  // Color palette
  icoEntry.writeUInt8(0, 3);  // Reserved
  icoEntry.writeUInt16LE(1, 4);  // Color planes
  icoEntry.writeUInt16LE(32, 6); // Bits per pixel
  icoEntry.writeUInt32LE(pngBuffer.length, 8); // Image size
  icoEntry.writeUInt32LE(22, 12); // Offset to image data

  const icoBuffer = Buffer.concat([icoHeader, icoEntry, pngBuffer]);
  fs.writeFileSync(path.join(iconsDir, 'icon.ico'), icoBuffer);
  console.log('Generated icon.ico');

  // Copy for ICNS (macOS will need proper icns but this is for Windows primarily)
  fs.copyFileSync(path.join(iconsDir, '128x128@2x.png'), path.join(iconsDir, 'icon.icns'));
  console.log('Generated icon.icns (placeholder)');
}

generateIcons().catch(console.error);
