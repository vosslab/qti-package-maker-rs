// qti-package-maker RDKit CanvasSource shim, ABI v1.
#include "qti_rdkit_shim.h"

#include <GraphMol/MolDraw2D/MolDraw2DCairo.h>
#include <GraphMol/MolDraw2D/MolDraw2DHelpers.h>
#include <GraphMol/SmilesParse/SmilesParse.h>
#include <GraphMol/Substruct/SubstructMatch.h>

#include <cstdlib>
#include <cstring>
#include <map>
#include <memory>
#include <stdexcept>
#include <string>
#include <vector>

namespace {

char *copy_error(const std::string &message) {
  auto *result = static_cast<char *>(std::malloc(message.size() + 1));
  if (result != nullptr) std::memcpy(result, message.c_str(), message.size() + 1);
  return result;
}

void add_peptide_bonds(const RDKit::ROMol &molecule, std::vector<int> *bonds) {
  std::unique_ptr<RDKit::RWMol> pattern(RDKit::SmartsToMol("CC(=O)NC"));
  if (!pattern) throw std::runtime_error("RDKit could not construct peptide SMARTS");
  for (const auto &match : RDKit::SubstructMatch(molecule, *pattern)) {
    const auto *bond = molecule.getBondBetweenAtoms(match[1].second, match[3].second);
    if (bond != nullptr) bonds->push_back(static_cast<int>(bond->getIdx()));
  }
}

}  // namespace

extern "C" unsigned int qti_rdkit_shim_abi_version() { return 1; }

extern "C" int qti_rdkit_render_png(
    const char *smiles, size_t smiles_length, const char *legend,
    size_t legend_length, unsigned int width, unsigned int height,
    bool explicit_methyl, const int *atom_indices, size_t atom_count,
    const int *bond_indices, size_t bond_count, const double *highlight_rgb,
    bool peptide_bonds, unsigned char **png, size_t *png_length, char **error) {
  if (png == nullptr || png_length == nullptr || error == nullptr || smiles == nullptr ||
      legend == nullptr || (atom_count != 0 && atom_indices == nullptr) ||
      (bond_count != 0 && bond_indices == nullptr)) {
    return 2;
  }
  *png = nullptr;
  *png_length = 0;
  *error = nullptr;
  try {
    std::unique_ptr<RDKit::RWMol> molecule(
        RDKit::SmilesToMol(std::string(smiles, smiles_length)));
    if (!molecule) {
      *error = copy_error("RDKit could not parse SMILES");
      return 3;
    }
    std::vector<int> atoms;
    std::vector<int> bonds;
    if (atom_count != 0) atoms.assign(atom_indices, atom_indices + atom_count);
    if (bond_count != 0) bonds.assign(bond_indices, bond_indices + bond_count);
    if (peptide_bonds) add_peptide_bonds(*molecule, &bonds);
    for (const int index : atoms) {
      if (index < 0 || static_cast<unsigned int>(index) >= molecule->getNumAtoms()) {
        *error = copy_error("RDKit atom highlight index is outside the molecule");
        return 4;
      }
    }
    for (const int index : bonds) {
      if (index < 0 || static_cast<unsigned int>(index) >= molecule->getNumBonds()) {
        *error = copy_error("RDKit bond highlight index is outside the molecule");
        return 5;
      }
    }
    if (peptide_bonds && bonds.empty()) {
      *error = copy_error("RDKit peptide canvas contains no matching peptide bonds");
      return 6;
    }
    RDKit::MolDraw2DCairo drawer(width, height);
    drawer.drawOptions().explicitMethyl = explicit_methyl;
    std::map<int, RDKit::DrawColour> atom_colours;
    std::map<int, RDKit::DrawColour> bond_colours;
    if (highlight_rgb != nullptr) {
      const RDKit::DrawColour colour(highlight_rgb[0], highlight_rgb[1], highlight_rgb[2]);
      for (const int index : atoms) atom_colours.emplace(index, colour);
      for (const int index : bonds) bond_colours.emplace(index, colour);
    }
    drawer.drawMolecule(*molecule, std::string(legend, legend_length), &atoms, &bonds,
                        &atom_colours, &bond_colours);
    drawer.finishDrawing();
    const std::string drawing = drawer.getDrawingText();
    auto *result = static_cast<unsigned char *>(std::malloc(drawing.size()));
    if (result == nullptr) {
      *error = copy_error("RDKit drawing allocation failed");
      return 7;
    }
    std::memcpy(result, drawing.data(), drawing.size());
    *png = result;
    *png_length = drawing.size();
    return 0;
  } catch (const std::exception &exception) {
    *error = copy_error(exception.what());
    return 8;
  }
}

extern "C" void qti_rdkit_free_bytes(unsigned char *value) { std::free(value); }
extern "C" void qti_rdkit_free_error(char *value) { std::free(value); }
