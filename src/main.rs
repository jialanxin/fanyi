use anyhow::Result;
use reqwest::get;
use serde_json::Value;
use ansi_term::Color;
use serde::Deserialize;
use quick_xml::de::from_str;

async fn youdao(words:&str)->Result<()> {
    let url = String::from("http://fanyi.youdao.com/openapi.do?keyfrom=node-fanyi&key=110811608&type=data&doctype=json&version=1.1&q=");
    let url = url+words;
    let resp = get(url).await?.text().await?;
    let result:Value = serde_json::from_str(resp.as_str())?;
    print_youdao(result)?;
    Ok(())
}

fn print_basic(data:&Value)->Result<()>{
    let query = data["query"].to_string().trim_matches('\"').to_string();
    let basic:&Value = &data["basic"];
    let suffix = Color::RGB(128,128,128).paint("  ~  fanyi.youdao.com");
    if let Value::Null = basic{
        println!("{}{}",query,suffix);
        return Ok(())
    }
    let phonetic = &basic["phonetic"];
    if let Value::Null = phonetic{
        println!("{}{}",query,suffix);
    }else{
        let phonetic = phonetic.to_string();
        let phonetic = phonetic.trim_matches('\"');
        let phonetic = "[ ".to_string()+phonetic+" ]";
        println!("{}   {} {}",query,Color::RGB(255,0,255).paint(phonetic),suffix)
    }
    print!("\n");
    let explains = &basic["explains"];
    if let Value::Array(vec) = explains{
        for item in vec{
            if let Value::String(exp) = item{
                let exp = exp.trim_matches('\"');
                let exp = Color::Green.paint(exp);
                println!("{}{}",Color::RGB(128,128,128).paint(" - "),exp);
            };
        }

    }
    Ok(())
}
fn print_sentences(data:&Value)->Result<()>{
    print!("\n");
    let query = data["query"].to_string().trim_matches('\"').to_string();
    let web = &data["web"];
    if let Value::Array(vec) = web{
        if vec.len() != 0{
            for (i,sen) in vec.iter().enumerate(){
                let key = &sen["key"];
                if let Value::String(key) = key{
                    let key = key.trim_matches('\"');
                    print!("{}. ",i+1);
                    highlight(key, &query)?;
                }
                let value = &sen["value"];
                if let Value::Array(vec) = value{
                    let result:Vec<String> = vec.iter().map(|item|item.to_string().trim_matches('\"').to_string()).collect();
                    let mut value = String::from("   ");
                    for item in result{
                        value.push_str(&item);
                        value.push_str(",");
                    }
                    println!("{}",Color::Cyan.paint(value));
                }
            }
        }
    }
    Ok(())
}

fn highlight(key:&str,query:&str)->Result<()>{
    let result:Vec<&str> =  key.split(query).collect();
    for (i,item) in result.iter().enumerate(){
        print!("{}",Color::RGB(128,128,128).paint(*item));
        if i != result.len()-1{
            print!("{}",Color::Yellow.paint(query));
        }else{
            print!("\n")
        }

    }
    Ok(())
}

fn print_youdao(data:Value)->Result<()>{
    print_basic(&data)?;
    print_sentences(&data)?;
    println!("\n   {}\n",Color::RGB(128,128,128).paint("------"));
    Ok(())
}

async fn iciba(word:String)->Result<()>{
    let url = String::from("http://dict-co.iciba.com/api/dictionary.php?key=D191EBD014295E913574E1EAF8E06666&w=");
    let url = url+&word;
    let resp = get(url).await?.text().await?;
    let dict:Dict = from_str(&resp)?;
    // println!("{:#?}",dict);
    let mut key = String::from("");
    let mut ps_count = 0;
    let mut pos_count = 0;
    let mut sent_count = 0;
    for item in dict.pses{   
        match item {
            DictInternal::KEY(key_string) => {
                key = key_string;
                print!("{}   ",&key);
            },
            DictInternal::PS(ps) =>{
                if ps_count == 0{
                    let ps_string = String::from("英 [ ");
                    let ps_string = ps_string+&ps+" ]  ";
                    print!("{}",Color::RGB(255,0,255).paint(ps_string));
                    ps_count += 1;
                }else{
                    let ps_string = String::from("美 [ ");
                    let ps_string = ps_string+&ps+" ]  ";
                    print!("{}",Color::RGB(255,0,255).paint(ps_string));
                    print!("{}",Color::RGB(128,128,128).paint("  ~  iciba.com\n"));
                }
            },
            DictInternal::POS(pos) =>{
                if pos_count == 0{
                    print!("\n");
                    pos_count += 1;
                }
                print!(" - {}",Color::Green.paint(pos));
            },
            DictInternal::ACCEPTATION(acce)=>{
                print!("  {}\n",Color::Green.paint(acce));
            },
            DictInternal::Sent(sent)=>{
                if sent_count == 0{
                    print!("\n");
                }
                sent_count += 1;
                let orig:String = sent.orig;
                let trans:String = sent.trans;
                print!(" {}. ",sent_count);
                highlight(&orig, &key)?;
                print!("    {}\n",Color::Cyan.paint(trans));
            }
            _ => ()
        }
    }
    print!("\n    {}\n\n",Color::RGB(128,128,128).paint("------"));
    Ok(())
}

#[derive(Debug, Deserialize, PartialEq)]
struct Sent{
    orig:String,
    trans:String,
}
#[derive(Debug, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum DictInternal{
    KEY(String),
    PS(String),
    PRON(String),
    POS(String),
    ACCEPTATION(String),
    FY(String),
    Sent(Sent)

}
#[derive(Debug, Deserialize, PartialEq)]
struct Dict{
    #[serde(rename = "$value", default)]
    pses: Vec<DictInternal>
}

#[tokio::main]
async fn main()->Result<()> {
    let args:Vec<String> = std::env::args().collect();
    // println!("{:?}", args);
    let words = &args[1..];
    // println!("{:?}", words);
    let mut input  = String::from("");
    for word in words{
        input.push_str(word);
        input.push_str(" ");
    }
    // println!("{:?}",input);
    // let input = "good";
    let input_2 = input.clone();
    let iciba_task = tokio::spawn(async move {iciba(input_2).await});
    // youdao(&input).await?;
    iciba_task.await?;
    Ok(())
}

