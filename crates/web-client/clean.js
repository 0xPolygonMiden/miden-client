import { glob } from 'glob';
import { rimraf } from 'rimraf';

// Use glob to find all directories or files matching 'dist/wasm*'
glob('dist/wasm*', (err, files) => {
  if (err) {
    console.error('Error finding files:', err);
    return;
  }

  // Iterate through the matched files/directories and delete them
  files.forEach((file) => {
    rimraf(file, (rimrafErr) => {
      if (rimrafErr) {
        console.error(`Error deleting ${file}:`, rimrafErr);
      } else {
        console.log(`Deleted: ${file}`);
      }
    });
  });
});
