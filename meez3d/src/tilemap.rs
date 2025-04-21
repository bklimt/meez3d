use std::cmp::Ordering;
use std::num::ParseIntError;
use std::ops::{Index, IndexMut};
use std::path::Path;
use std::str::FromStr;

use crate::filemanager::FileManager;
use crate::geometry::{Point, Rect};
use crate::imagemanager::ImageLoader;
use crate::properties::{PropertiesXml, PropertyMap};
use crate::rendercontext::{RenderContext, RenderLayer};
use crate::sprite::{Animation, Sprite};
use crate::tileset::{LocalTileIndex, TileProperties, TileSet};
use crate::utils::Color;

use anyhow::{anyhow, bail, Context, Result};
use log::info;
use num_traits::Zero;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct TileSetSourceXml {
    #[serde(rename = "@source")]
    source: String,

    #[serde(rename = "@firstgid")]
    firstgid: usize,
}

#[derive(Debug, Deserialize)]
struct DataXml {
    #[serde(rename = "@encoding")]
    _encoding: String,

    #[serde(rename = "$value")]
    data: String,
}

#[derive(Debug, Deserialize)]
struct LayerXml {
    #[serde(rename = "@id")]
    id: u32,
    #[serde(rename = "@name")]
    name: String,
    #[serde(rename = "@width")]
    width: u32,
    #[serde(rename = "@height")]
    height: u32,

    data: DataXml,

    properties: Option<PropertiesXml>,
}

#[derive(Debug, Deserialize)]
struct ImageXml {
    #[serde(rename = "@source")]
    source: String,
}

#[derive(Debug, Deserialize)]
struct ImageLayerXml {
    #[serde(rename = "@id")]
    _id: i32,
    #[serde(rename = "@offsetx")]
    _offsetx: Option<String>,
    #[serde(rename = "@offsety")]
    _offsety: Option<String>,

    image: ImageXml,
}

#[derive(Debug, Deserialize)]
struct ObjectXml {
    #[serde(rename = "@id")]
    id: i32,
    #[serde(rename = "@x")]
    x: i32,
    #[serde(rename = "@y")]
    y: i32,
    #[serde(rename = "@width")]
    width: Option<i32>,
    #[serde(rename = "@height")]
    height: Option<i32>,
    #[serde(rename = "@gid")]
    gid: Option<u32>,

    properties: Option<PropertiesXml>,
}

#[derive(Debug, Deserialize)]
struct ObjectGroupXml {
    #[serde(default)]
    object: Vec<ObjectXml>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum TileMapXmlField {
    TileSet(TileSetSourceXml),
    ObjectGroup(ObjectGroupXml),
    Layer(LayerXml),
    ImageLayer(ImageLayerXml),
}

fn default_backgroundcolor() -> String {
    "#000000".to_string()
}

#[derive(Debug, Deserialize)]
struct TileMapXml {
    #[serde(rename = "@width")]
    width: i32,
    #[serde(rename = "@height")]
    height: i32,
    #[serde(rename = "@tilewidth")]
    tilewidth: i32,
    #[serde(rename = "@tileheight")]
    tileheight: i32,
    #[serde(rename = "@backgroundcolor", default = "default_backgroundcolor")]
    backgroundcolor: String,

    #[serde(rename = "$value")]
    fields: Vec<TileMapXmlField>,

    properties: Option<PropertiesXml>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TileIndex(usize);

impl From<TileIndex> for usize {
    fn from(value: TileIndex) -> Self {
        value.0
    }
}

impl From<usize> for TileIndex {
    fn from(value: usize) -> Self {
        TileIndex(value)
    }
}

impl FromStr for TileIndex {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(TileIndex(s.parse::<usize>()?))
    }
}

struct ImageLayer {
    surface: Sprite,
}

impl ImageLayer {
    fn from_xml(
        xml: ImageLayerXml,
        path: &Path,
        images: &mut dyn ImageLoader,
    ) -> Result<ImageLayer> {
        let path = path
            .parent()
            .context("xml file is root")?
            .join(xml.image.source);
        let surface = images.load_sprite(&path)?;
        Ok(ImageLayer { surface })
    }
}

struct TileLayer {
    _id: u32,
    _name: String,
    _width: u32,
    _height: u32,
    data: Vec<Vec<TileIndex>>,
    player: bool,
}

impl TileLayer {
    fn from_xml(xml: LayerXml) -> Result<TileLayer> {
        let id = xml.id;
        let name = xml.name;
        let width = xml.width;
        let height = xml.height;

        let props: Option<PropertyMap> = xml.properties.map(|x| x.try_into()).transpose()?;
        let props = props.unwrap_or_default();
        let player = props.get_bool("player")?.unwrap_or(false);

        let mut data = Vec::new();
        for line in xml.data.data.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let mut row = Vec::new();
            for part in line.split(',') {
                if part.is_empty() {
                    continue;
                }
                row.push(part.parse().context(format!("parsing {:?}", part))?);
            }
            if row.len() as u32 != width {
                bail!("row len = {}, but width = {}", row.len(), width);
            }
            data.push(row);
        }
        if data.len() as u32 != height {
            bail!("row data height = {}, but height = {}", data.len(), height);
        }

        Ok(TileLayer {
            _id: id,
            _name: name,
            _width: width,
            _height: height,
            data,
            player,
        })
    }

    fn get(&self, row: usize, col: usize) -> Option<&TileIndex> {
        self.data.get(row).and_then(|r| r.get(col))
    }

    fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut TileIndex> {
        self.data.get_mut(row).and_then(|r| r.get_mut(col))
    }
}

impl Index<(usize, usize)> for TileLayer {
    type Output = TileIndex;

    fn index(&self, index: (usize, usize)) -> &Self::Output {
        self.get(index.0, index.1)
            .with_context(|| anyhow!("indices must be valid: ({}, {})", index.0, index.1))
            .expect("indices must be valid")
    }
}

impl IndexMut<(usize, usize)> for TileLayer {
    fn index_mut(&mut self, index: (usize, usize)) -> &mut Self::Output {
        self.get_mut(index.0, index.1)
            .expect("indices must be valid")
    }
}

enum Layer {
    Tile(TileLayer),
    Image(ImageLayer),
}

#[derive(Debug, Clone, Copy)]
pub enum Overflow {
    Oscillate,
    Wrap,
    Clamp,
}

impl FromStr for Overflow {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "oscillate" => Ok(Overflow::Oscillate),
            "wrap" => Ok(Overflow::Wrap),
            "clamp" => Ok(Overflow::Clamp),
            _ => Err(anyhow!("invalid overflow type: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ConveyorDirection {
    Left,
    Right,
}

impl FromStr for ConveyorDirection {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "W" => Ok(ConveyorDirection::Left),
            "E" => Ok(ConveyorDirection::Right),
            _ => Err(anyhow!("invalid conveyor direction: {}", s)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum ButtonType {
    OneShot,
    Toggle,
    Momentary,
    Smart,
}

impl FromStr for ButtonType {
    type Err = anyhow::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "oneshot" => ButtonType::OneShot,
            "toggle" => ButtonType::Toggle,
            "momentary" => ButtonType::Momentary,
            "smart" => ButtonType::Smart,
            _ => bail!("invalid button type: {}", s),
        })
    }
}

#[derive(Debug)]
pub struct MapObjectProperties {
    // Tiles
    pub solid: bool,
    // Map Areas
    pub preferred_x: Option<i32>,
    pub preferred_y: Option<i32>,
    // UI elements
    pub uibutton: bool,
    pub action: Option<String>,
    pub label: String,
    _raw: PropertyMap,
}

impl TryFrom<PropertyMap> for MapObjectProperties {
    type Error = anyhow::Error;
    fn try_from(properties: PropertyMap) -> Result<Self> {
        Ok(MapObjectProperties {
            solid: properties.get_bool("solid")?.unwrap_or(false),
            preferred_x: properties.get_int("preferred_x")?,
            preferred_y: properties.get_int("preferred_y")?,
            uibutton: properties.get_bool("uibutton")?.unwrap_or(false),
            label: properties.get_string("label")?.unwrap_or("").to_string(),
            action: properties.get_string("action")?.map(str::to_string),
            _raw: properties,
        })
    }
}

pub struct MapObject {
    pub id: i32,
    pub gid: Option<TileIndex>,
    pub position: Rect<i32>,
    pub properties: MapObjectProperties,
}

impl MapObject {
    fn new(xml: ObjectXml, tilesets: &TileSetList) -> Result<MapObject> {
        let id = xml.id;
        let x = xml.x;
        let mut y = xml.y;
        let width = xml.width.unwrap_or(0);
        let height = xml.height.unwrap_or(0);
        let mut properties: PropertyMap = xml
            .properties
            .map(|x| x.try_into())
            .transpose()?
            .unwrap_or_default();
        let gid = xml.gid.map(|index| (index as usize).into());

        if let Some(gid) = gid {
            let (tileset, tile_id) = tilesets.lookup(gid);
            let defaults = tileset.get_tile_properties(tile_id);
            if let Some(props) = defaults {
                properties.set_defaults(&props.raw);
            }
            // For some reason, the position is the bottom left sometimes?
            y -= height;
        }

        let position = Rect {
            x,
            y,
            w: width,
            h: height,
        };

        let properties = properties.try_into()?;

        Ok(MapObject {
            id,
            gid,
            position,
            properties,
        })
    }
}

struct TileSetList {
    tilesets: Vec<TileSet>,
}

impl TileSetList {
    fn new() -> Self {
        Self {
            tilesets: Vec::new(),
        }
    }

    fn add(&mut self, tileset: TileSet) {
        self.tilesets.push(tileset);
        self.tilesets.sort_by_key(|tileset| tileset.gid_sort_key());
    }

    fn lookup(&self, tile_gid: TileIndex) -> (&TileSet, LocalTileIndex) {
        for tileset in self.tilesets.iter() {
            if let Some(tile_id) = tileset.get_local_tile_index(tile_gid) {
                return (tileset, tile_id);
            }
        }
        panic!("invalid tile_gid {:?}", tile_gid);
    }
}

pub struct TileMapProperties {
    pub dark: bool,
    pub gravity: Option<i32>,
    pub cancel_action: String,
}

impl TryFrom<PropertyMap> for TileMapProperties {
    type Error = anyhow::Error;
    fn try_from(properties: PropertyMap) -> Result<Self> {
        Ok(TileMapProperties {
            dark: properties.get_bool("is_dark")?.unwrap_or(false),
            gravity: properties.get_int("gravity")?.map(|x| x / 16),
            cancel_action: properties
                .get_string("cancel_action")?
                .unwrap_or("pop")
                .to_string(),
        })
    }
}

pub struct TileMap {
    pub width: i32,
    pub height: i32,
    pub tilewidth: i32,
    pub tileheight: i32,
    backgroundcolor: Color,
    tilesets: TileSetList,
    layers: Vec<Layer>,
    player_layer: Option<i32>, // TODO: Should just be i32.
    pub objects: Vec<MapObject>,
    pub properties: TileMapProperties,
}

impl TileMap {
    pub fn from_file(
        path: &Path,
        files: &FileManager,
        images: &mut dyn ImageLoader,
    ) -> Result<TileMap> {
        info!("loading tilemap from {:?}", path);
        let text = files
            .read_to_string(path)
            .map_err(|e| anyhow!("unable to open {:?}: {}", path, e))?;
        let xml = quick_xml::de::from_str::<TileMapXml>(&text)?;
        Self::from_xml(xml, path, files, images)
    }

    fn from_xml(
        xml: TileMapXml,
        path: &Path,
        files: &FileManager,
        images: &mut dyn ImageLoader,
    ) -> Result<TileMap> {
        let width = xml.width;
        let height = xml.height;
        let tilewidth = xml.tilewidth;
        let tileheight = xml.tileheight;
        let backgroundcolor = xml.backgroundcolor.parse().context(format!(
            "parsing background color {:?}",
            &xml.backgroundcolor
        ))?;

        let mut tilesets = TileSetList::new();
        for field in xml.fields.iter() {
            if let TileMapXmlField::TileSet(tileset) = field {
                let firstgid = tileset.firstgid.into();
                let tileset_path = path
                    .parent()
                    .context("cannot load root as map")?
                    .join(tileset.source.clone());
                let tileset = TileSet::from_file(&tileset_path, firstgid, files, images)?;
                tilesets.add(tileset);
            }
        }
        if tilesets.tilesets.is_empty() {
            bail!("at least one tileset must be present");
        }

        let mut player_layer: Option<i32> = None;
        let mut layers = Vec::new();
        let mut objects: Vec<MapObject> = Vec::new();
        for field in xml.fields {
            match field {
                TileMapXmlField::Layer(layer) => {
                    let layer = TileLayer::from_xml(layer)?;
                    if layer.player {
                        if player_layer.is_some() {
                            bail!("too many player layers");
                        }
                        player_layer = Some(layers.len() as i32);
                    }
                    layers.push(Layer::Tile(layer));
                }
                TileMapXmlField::ImageLayer(layer) => {
                    layers.push(Layer::Image(ImageLayer::from_xml(layer, path, images)?));
                }
                TileMapXmlField::ObjectGroup(group) => {
                    for object in group.object {
                        objects.push(MapObject::new(object, &tilesets)?);
                    }
                }
                _ => {}
            }
        }

        let properties = if let Some(props) = xml.properties {
            props.try_into()?
        } else {
            PropertyMap::new()
        };

        let properties = properties.try_into()?;

        Ok(TileMap {
            width,
            height,
            tilewidth,
            tileheight,
            backgroundcolor,
            tilesets,
            layers,
            player_layer,
            objects,
            properties,
        })
    }

    fn draw_image_layer(
        &self,
        layer: &ImageLayer,
        context: &mut RenderContext,
        render_layer: RenderLayer,
        _dest: Rect<i32>,
        offset: Point<i32>,
    ) {
        let dest = Rect {
            x: offset.x,
            y: offset.y,
            w: layer.surface.area.w,
            h: layer.surface.area.h,
        };
        let source = Rect {
            x: 0,
            y: 0,
            w: layer.surface.area.w,
            h: layer.surface.area.h,
        };
        context.draw(layer.surface, render_layer, dest, source);
    }

    fn draw_tile_layer(
        &self,
        layer: &TileLayer,
        context: &mut RenderContext,
        render_layer: RenderLayer,
        dest: Rect<i32>,
        offset: Point<i32>,
    ) {
        let offset_x = offset.x;
        let offset_y = offset.y;
        let tileheight = self.tileheight;
        let tilewidth = self.tilewidth;

        let dest_h = dest.h as f32;
        let dest_w = dest.w as f32;
        let tileheight_f = tileheight as f32;
        let tilewidth_f = tilewidth as f32;

        let row_count = (dest_h / tileheight_f).ceil() as i32 + 1;
        let col_count = (dest_w / tilewidth_f).ceil() as i32 + 1;

        let start_row = (-(offset_y / tileheight)).max(0);
        let end_row = (start_row + row_count).min(self.height);

        let start_col = (-(offset_x / tilewidth)).max(0);
        let end_col = (start_col + col_count).min(self.width);

        for row in start_row..end_row {
            for col in start_col..end_col {
                // Compute what to draw where.
                let index = layer
                    .data
                    .get(row as usize)
                    .expect("size was checked at init")
                    .get(col as usize)
                    .expect("size was checked at init");
                let index = *index;
                if index.0 == 0 {
                    continue;
                }

                let (tileset, tile_id) = self.tilesets.lookup(index);

                let mut source = tileset.get_source_rect(tile_id);
                let mut pos_x = tilewidth * col + dest.x + offset_x;
                let mut pos_y = tileheight * row + dest.y + offset_y;

                // If it's off the top/left side, trim it.
                if pos_x < dest.x {
                    let extra = dest.left() - pos_x;
                    source.x += extra;
                    source.w -= extra;
                    pos_x = dest.x;
                }
                if pos_y < dest.y {
                    let extra = dest.top() - pos_y;
                    source.y += extra;
                    source.h -= extra;
                    pos_y = dest.y;
                }
                if source.w <= 0 || source.h <= 0 {
                    continue;
                }

                // If it's off the right/bottom side, trim it.
                let pos_right = pos_x + tilewidth;
                if pos_right >= dest.right() {
                    source.w -= (pos_right - dest.right());
                }
                if source.w <= 0 {
                    continue;
                }
                let pos_bottom = pos_y + tileheight;
                if pos_bottom >= dest.bottom() {
                    source.h -= (pos_bottom - dest.bottom());
                }
                if source.h <= 0 {
                    continue;
                }

                // TODO: Trim the dest separately so that we don't have subpixel rounding errors.

                // Draw the rest of the turtle.
                let destination = Rect {
                    x: pos_x,
                    y: pos_y,
                    w: source.w,
                    h: source.h,
                };
                if let Some(animation) = self.get_animation(index) {
                    animation.blit(context, render_layer, destination, false);
                } else {
                    context.draw(tileset.sprite, render_layer, destination, source);
                }
            }
        }
    }

    fn draw_layer(
        &self,
        layer: &Layer,
        context: &mut RenderContext,
        render_layer: RenderLayer,
        dest: Rect<i32>,
        offset: Point<i32>,
    ) {
        match layer {
            Layer::Image(layer) => {
                self.draw_image_layer(layer, context, render_layer, dest, offset)
            }
            Layer::Tile(layer) => self.draw_tile_layer(layer, context, render_layer, dest, offset),
        }
    }

    pub fn draw_background(
        &self,
        context: &mut RenderContext,
        render_layer: RenderLayer,
        dest: Rect<i32>,
        offset: Point<i32>,
    ) {
        context.fill_rect(dest, render_layer, self.backgroundcolor);
        for layer in self.layers.iter() {
            self.draw_layer(layer, context, render_layer, dest, offset);
            if let Layer::Tile(TileLayer { player: true, .. }) = layer {
                return;
            }
        }
    }

    pub fn draw_foreground(
        &self,
        context: &mut RenderContext,
        render_layer: RenderLayer,
        dest: Rect<i32>,
        offset: Point<i32>,
    ) {
        if self.player_layer.is_none() {
            return;
        }
        let mut drawing = false;
        for layer in self.layers.iter() {
            if drawing {
                self.draw_layer(layer, context, render_layer, dest, offset);
            }
            if let Layer::Tile(TileLayer { player: true, .. }) = layer {
                drawing = true;
            }
        }
    }

    /*
    fn get_rect(&self, row: i32, col: i32) -> Rect<Pixels> {
        Rect {
            x: self.tilewidth * col,
            y: self.tileheight * row,
            w: self.tilewidth,
            h: self.tileheight,
        }
    }
    pub fn get_preferred_view(
        &self,
        player_rect: Rect<Subpixels>,
    ) -> (Option<Subpixels>, Option<Subpixels>) {
        let mut preferred_x = None;
        let mut preferred_y = None;
        for obj in self.objects.iter() {
            if obj.gid.is_some() {
                continue;
            }
            if !player_rect.intersects(obj.position.into()) {
                continue;
            }
            if let Some(p_x) = obj.properties.preferred_x {
                preferred_x = Some(p_x.as_subpixels());
            }
            if let Some(p_y) = obj.properties.preferred_y {
                preferred_y = Some(p_y.as_subpixels());
            }
        }
        (preferred_x, preferred_y)
    }

    pub fn draw_tile(
        &self,
        context: &mut RenderContext,
        tile_gid: TileIndex,
        layer: RenderLayer,
        dest: Rect<Subpixels>,
    ) {
        let (tileset, tile_id) = self.tilesets.lookup(tile_gid);
        let src = tileset.get_source_rect(tile_id);
        context.draw(tileset.sprite, layer, dest, src);
    }
    */

    pub fn get_animation(&self, tile_gid: TileIndex) -> Option<&Animation> {
        let (tileset, tile_id) = self.tilesets.lookup(tile_gid);
        tileset.animations.get(tile_id)
    }

    /*
    pub fn get_tile_properties(&self, tile_gid: TileIndex) -> Option<&TileProperties> {
        let (tileset, tile_id) = self.tilesets.lookup(tile_gid);
        tileset.get_tile_properties(tile_id)
    }
    */
}
